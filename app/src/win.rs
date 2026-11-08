//! Petites retouches Win32 que winit ne permet pas d'exprimer directement.
//!
//! Les fenêtres enfants sont transparentes, mais Windows 11 leur ajoute un
//! liseré clair et les place au-dessus des fenêtres normales. On corrige ces
//! deux détails par le biais de l'API DWM / User32.

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED,
    DwmSetWindowAttribute,
};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetClientRect, GetShellWindow, GetTopWindow, GetWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, GWL_STYLE, GW_HWNDNEXT, HWND_BOTTOM, HWND_TOPMOST,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER,
    SW_SHOWNOACTIVATE, SetWindowPos, WS_CAPTION, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
};

unsafe extern "system" {
    fn CreateRectRgn(x1: i32, y1: i32, x2: i32, y2: i32) -> HWND;
    fn SetWindowRgn(hwnd: HWND, hrgn: HWND, b_redraw: i32) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn find_window(title: &str) -> Option<HWND> {
    let title = wide(title);
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    (hwnd != 0).then_some(hwnd)
}

/// Fenêtre du bureau qui héberge les icônes (`SHELLDLL_DefView`).
///
/// Jusqu'à Windows 11 23H2, les icônes vivent dans un `WorkerW` distinct ;
/// depuis 24H2, c'est `Progman` qui les héberge directement.
fn desktop_icons_host() -> Option<HWND> {
    unsafe {
        let shell = GetShellWindow();
        if shell == 0 {
            return None;
        }

        let def_view = wide("SHELLDLL_DefView");
        if FindWindowExW(shell, 0, def_view.as_ptr(), std::ptr::null()) != 0 {
            return Some(shell);
        }

        let worker_w = wide("WorkerW");
        let mut worker: HWND = 0;
        loop {
            worker = FindWindowExW(0, worker, worker_w.as_ptr(), std::ptr::null());
            if worker == 0 {
                return None;
            }
            if FindWindowExW(worker, 0, def_view.as_ptr(), std::ptr::null()) != 0
                && same_process(shell, worker)
            {
                return Some(worker);
            }
        }
    }
}

fn same_process(a: HWND, b: HWND) -> bool {
    let mut process_a = 0;
    let mut process_b = 0;
    unsafe {
        GetWindowThreadProcessId(a, &mut process_a);
        GetWindowThreadProcessId(b, &mut process_b);
    }
    process_a == process_b
}

/// Vrai si `upper` se trouve au-dessus de `lower` dans l'ordre Z.
fn is_above(upper: HWND, lower: HWND) -> bool {
    unsafe {
        let mut hwnd = GetTopWindow(0);
        while hwnd != 0 {
            if hwnd == upper {
                return true;
            }
            if hwnd == lower {
                return false;
            }
            hwnd = GetWindow(hwnd, GW_HWNDNEXT);
        }
    }
    false
}

/// Détecte l'état « Afficher le bureau » (Win + D, bouton de la barre des
/// tâches, aperçu Aero).
///
/// Windows ne minimise pas les fenêtres : il fait passer le bureau au-dessus
/// d'elles. On compare donc l'ordre Z du bureau à celui d'une fenêtre témoin
/// que l'on ne déplace jamais (l'hôte invisible).
pub fn is_show_desktop_active(reference_title: &str) -> bool {
    let Some(host) = desktop_icons_host() else {
        return false;
    };
    // Sur Windows 10 le WorkerW des icônes est masqué hors « Afficher le
    // bureau » ; sur 24H2 Progman reste visible et c'est l'ordre Z qui décide.
    if unsafe { IsWindowVisible(host) } == 0 {
        return false;
    }
    let Some(reference) = find_window(reference_title) else {
        return false;
    };
    is_above(host, reference)
}

/// Place la fenêtre au bas de la pile des fenêtres normales, ou au-dessus du
/// bureau lorsqu'il est affiché, pour qu'elle reste visible pendant Win + D.
pub fn set_desktop_z_order(title: &str, desktop_shown: bool) {
    if let Some(hwnd) = find_window(title) {
        unsafe {
            if desktop_shown {
                // Le bureau est remonté au premier plan : il faut passer devant
                // lui, sinon il recouvre le widget.
                if IsIconic(hwnd) != 0 || IsWindowVisible(hwnd) == 0 {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
            } else {
                SetWindowPos(
                    hwnd,
                    HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
            }
        }
    }
}

/// Applique une seule fois tous les réglages Win32 d'une fenêtre.
///
/// Ces opérations doivent rester ponctuelles : les répéter périodiquement
/// provoque un clignotement des bordures DWM des fenêtres transparentes.
/// Renvoie `false` tant que la fenêtre n'existe pas encore.
pub fn tune(title: &str) -> bool {
    let Some(hwnd) = find_window(title) else {
        return false;
    };
    tune_hwnd(hwnd);
    true
}

fn tune_hwnd(hwnd: HWND) {
    strip_window_chrome(hwnd);
    clip_top_chrome(hwnd);
    apply_toolwindow_style(hwnd);
}

/// Répare une fenêtre dont Windows a redessiné le cadre pendant une veille.
///
/// Au réveil, le compositeur peut restaurer la zone non cliente d'après le
/// style Win32 (`WS_CAPTION`) sans repasser par `WM_NCCALCSIZE` : la barre
/// plein écran affiche alors un vrai titre. On ne retouche la fenêtre que si
/// sa zone client ne couvre plus toute la surface, pour ne pas faire
/// clignoter les bordures DWM à chaque vérification.
///
/// Renvoie `true` si un cadre a été retiré.
pub fn repair(title: &str) -> bool {
    let Some(hwnd) = find_window(title) else {
        return false;
    };
    if !has_client_chrome(hwnd) {
        return false;
    }
    tune_hwnd(hwnd);
    true
}

/// Vrai si la zone client est plus petite que la fenêtre, c'est-à-dire si
/// Windows y a réintroduit un cadre (titre, bordure).
fn has_client_chrome(hwnd: HWND) -> bool {
    let mut window = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut client = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        if GetWindowRect(hwnd, &mut window) == 0 || GetClientRect(hwnd, &mut client) == 0 {
            return false;
        }
    }
    let width = window.right - window.left;
    let height = window.bottom - window.top;
    // Marge d'un pixel : les cadres réels font plusieurs dizaines de pixels.
    client.right < width - 1 || client.bottom < height - 1
}

/// Rétablit l'ordre d'empilement des trois fenêtres : l'horloge devant le
/// visualiseur, lui-même devant les bandes d'ombre.
///
/// Les trois fenêtres partagent la bande « toujours au fond », que Windows
/// réordonne librement (activation du bureau, application asynchrone des
/// niveaux de fenêtre par winit). Sans cela, la barre plein écran peut passer
/// devant l'horloge et le visualiseur et les assombrir. On ne touche à l'ordre
/// Z que s'il a bougé, pour éviter tout clignotement.
pub fn keep_widget_stack() {
    let clock = find_window("Miklfe");
    let visualizer = find_window("Miklfe visualizer");
    let bar = find_window("Miklfe bar");

    if let (Some(clock), Some(visualizer)) = (clock, visualizer) {
        if !is_above(clock, visualizer) {
            stack_under(visualizer, clock);
        }
    }
    if let (Some(visualizer), Some(bar)) = (visualizer, bar) {
        if !is_above(visualizer, bar) {
            stack_under(bar, visualizer);
        }
    }
}

/// Place `hwnd` juste sous `above` (qui le précède dans l'ordre Z).
fn stack_under(hwnd: HWND, above: HWND) {
    unsafe {
        SetWindowPos(
            hwnd,
            above,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}

/// Transforme la fenêtre en "ToolWindow" pour qu'elle n'apparaisse ni dans la
/// barre des tâches ni dans Alt + Tab.
fn apply_toolwindow_style(hwnd: HWND) {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_style = ex_style | (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) as isize;

        if ex_style != new_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);

            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_FRAMECHANGED
                    | SWP_NOACTIVATE
                    | SWP_NOZORDER,
            );
        }
    }
}

/// Supprime le liseré et les coins arrondis ajoutés par Windows 11 aux
/// fenêtres sans décoration, ainsi que les styles de cadre Win32.
///
/// winit masque le cadre en interceptant `WM_NCCALCSIZE`, mais conserve
/// `WS_CAPTION` dans le style. Si Windows recalcule la zone non cliente sans
/// passer par ce message (réveil après veille), il redessine un titre. On
/// retire donc aussi ces bits pour que le cadre ne puisse pas revenir.
fn strip_window_chrome(hwnd: HWND) {
    let border: u32 = DWMWA_COLOR_NONE;
    let policy: i32 = DWMNCRP_DISABLED;
    let chrome = (WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX) as isize;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &border as *const u32 as *const core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY as u32,
            &policy as *const i32 as *const core::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
        );
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let stripped = style & !chrome;
        if stripped != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, stripped);
        }
    }
}

/// Découpe la fenêtre pour faire disparaître un éventuel reliquat d'un
/// liseré en haut de la zone client (fenêtres DWM modernes).
fn clip_top_chrome(hwnd: HWND) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 2 || height <= 2 {
        return;
    }
    let region = unsafe { CreateRectRgn(0, 1, width, height) };
    if region != 0 {
        unsafe {
            SetWindowRgn(hwnd, region, 1);
            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED
                    | SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOACTIVATE
                    | SWP_NOZORDER,
            );
        }
    }
}