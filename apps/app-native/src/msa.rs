//! Окно входа Microsoft. Запускается отдельным процессом (`--msa-login URL`),
//! чтобы WebKit не попадал в основное окно. Код авторизации печатается в stdout: `CODE=...`.

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;

const REDIRECT: &str = "https://login.live.com/oauth20_desktop.srf";

fn code_from(u: &str) -> Option<String> {
    if !u.starts_with(REDIRECT) {
        return None;
    }
    let parsed = url::Url::parse(u).ok()?;
    parsed.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.to_string())
}

fn finish(code: String) -> ! {
    println!("CODE={code}");
    std::process::exit(0);
}

pub fn run(auth_url: &str) -> ! {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Вход в Microsoft — TrollLauncher")
        .with_inner_size(tao::dpi::LogicalSize::new(560.0, 720.0))
        .build(&event_loop)
        .expect("window");

    let builder = wry::WebViewBuilder::new().with_url(auth_url).with_navigation_handler(|u: String| {
        if let Some(code) = code_from(&u) {
            finish(code);
        }
        true
    });

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().expect("vbox")).expect("webview")
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window).expect("webview");

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(150));
        if let Ok(u) = webview.url()
            && let Some(code) = code_from(&u)
        {
            finish(code);
        }
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            std::process::exit(1);
        }
    })
}
