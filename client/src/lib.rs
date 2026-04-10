use leptos::*;
use leptos_meta::*;
use leptos_router::*;

mod components;
mod pages;
use components::{Header, Footer};

use pages::HomePage;
use pages::SchedulePage;
use pages::CalendarPage;
use pages::LoginPage;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Title text="W9 Daily Reminders"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        <Stylesheet id="voxel" href="/pkg/w9-daily-reminders-client.css"/>
        <Router>
            <div class="app-container">
                <Header/>
                <main class="main-content">
                    <Routes>
                        <Route path="home" view=HomePage/>
                        <Route path="schedule" view=SchedulePage/>
                        <Route path="calendar" view=CalendarPage/>
                        <Route path="login" view=LoginPage/>
                    </Routes>
                </main>
                <Footer/>
            </div>
        </Router>
    }
}
