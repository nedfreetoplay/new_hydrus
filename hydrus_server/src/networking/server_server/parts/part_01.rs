impl ServerResourceTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// `root.putChild`
    pub fn put_child(&mut self, path: &'static str, route: ServerRoute) {
        if let Some((_, existing)) = self
            .children
            .iter_mut()
            .find(|(existing_path, _)| *existing_path == path)
        {
            *existing = route;
        } else {
            self.children.push((path, route));
        }
    }

    #[allow(dead_code)]
    pub fn get_child(&self, path: &str) -> Option<&ServerRoute> {
        self.children
            .iter()
            .find(|(existing_path, _)| *existing_path == path)
            .map(|(_, route)| route)
    }

    #[allow(dead_code)]
    pub fn children(&self) -> impl Iterator<Item = (&'static str, &ServerRoute)> {
        self.children
            .iter()
            .map(|(path, route)| (*path, route))
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}
