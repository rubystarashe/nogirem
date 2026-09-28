# Local patch: non-activating notification WebViews

Source: crates.io dioxus-desktop 0.7.10 (DioxusLabs/dioxus), MIT OR Apache-2.0.
One behavioral change in src/webview.rs: forward WindowBuilder.focused to Wry.with_focused.
Upstream forwarded focusability to the HWND, but left Wry focused=true. WebView2 then
activated the update notification despite focusable=false and SW_SHOWNOACTIVATE.
Main/interactive windows keep their existing default focus behavior.
Remove this patch when upstream exposes/honors the WebView focused option.
No registry cache is modified. Cargo.lock pins this local source.
