// Views for `reference/app/views/public_pages`.

#[derive(Clone, Copy)]
pub enum Page {
    About,
    Privacy,
    Terms,
}
impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Self::About => "Smartfire | About",
            Self::Privacy => "Smartfire | Privacy Policy",
            Self::Terms => "Smartfire | Terms of Service",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::About => {
                "What Smartfire is: self-hosted, open-source team chat run by the organization hosting each workspace."
            }
            Self::Privacy => {
                "Smartfire privacy policy: what each self-hosted workspace stores, who can see it, and how Google sign-in, Calendar, and Drive data is handled."
            }
            Self::Terms => {
                "Smartfire terms of service: the open-source software license and the rules for using a self-hosted workspace."
            }
        }
    }
}
