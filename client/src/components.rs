use leptos::*;
#[component] pub fn Header() -> impl IntoView {
    view! { <header class="voxel-header"><div class="header-content"><h1 class="logo-text">"W9"</h1><nav class="header-nav"><a href="/">"HOME"</a></nav></div></header> }
}
#[component] pub fn Footer() -> impl IntoView {
    view! { <footer class="voxel-footer"><div class="footer-content"><div class="footer-section"><h3>"W9"</h3><p>"W9 Labs"</p></div><div class="footer-section"><p>"© 2026 W9 Labs"</p></div></div></footer> }
}
