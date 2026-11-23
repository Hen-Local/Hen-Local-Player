pub struct Translator;
impl Translator {
    pub fn new() -> Self {
        Self
    }
    pub async fn load(&mut self) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn translate(
        &mut self,
        source: Option<&str>,
        target: &str,
        text: &str,
        context: Option<&str>,
    ) -> anyhow::Result<String> {
        todo!()
    }
}
