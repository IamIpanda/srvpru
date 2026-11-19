//! The composed landing page: it renders the components other plugins contribute without owning a route.

use crate::portal::Page;

#[distributed_slice(crate::portal::PORTAL_PAGES)]
static PAGE: Page = Page { 
    plugin: "srvpro::core", 
    route: "/", 
    export: "Playground", 
    topbar: true 
};
