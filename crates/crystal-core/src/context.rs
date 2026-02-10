#[derive(Debug, Clone)]
pub struct ClusterContext {
    pub name: String,
    pub namespace: String,
}

pub struct ContextResolver {
    active: Option<ClusterContext>,
}

impl ContextResolver {
    pub fn new() -> Self {
        Self { active: None }
    }

    pub fn resolve(&self) -> Option<&ClusterContext> {
        self.active.as_ref()
    }

    pub fn set_context(&mut self, ctx: ClusterContext) {
        self.active = Some(ctx);
    }

    pub fn set_namespace(&mut self, ns: &str) {
        if let Some(ref mut ctx) = self.active {
            ctx.namespace = ns.to_string();
        }
    }

    pub fn context_name(&self) -> Option<&str> {
        self.active.as_ref().map(|c| c.name.as_str())
    }

    pub fn namespace(&self) -> Option<&str> {
        self.active.as_ref().map(|c| c.namespace.as_str())
    }

    pub fn env_vars(&self) -> Vec<(String, String)> {
        match &self.active {
            Some(ctx) => {
                vec![("K8S_CONTEXT".into(), ctx.name.clone()), ("K8S_NAMESPACE".into(), ctx.namespace.clone())]
            }
            None => vec![],
        }
    }
}

impl Default for ContextResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
