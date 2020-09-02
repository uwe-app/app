use std::path::PathBuf;
use std::fs::{self, File};

use log::info;

use compiler::{BuildContext, Compiler, ParseData, parser::Parser};
use locale::Locales;

use crate::Result;

#[derive(Debug)]
pub struct Render<'a> {
    pub context: BuildContext,
    pub locales: &'a Locales,
}

impl Render<'_> {

    pub async fn render(&self) -> Result<()> {
        Ok(())
    }

    /*
    async fn compile(&self) -> Result<()> {
        let mut opts = self.context.options.clone();
        let mut ctx = self.context;

        let locale_map = opts.locales;

        let base_target = opts.target.clone();
        let mut previous_base = base_target.clone();

        for lang in locale_map.map.keys() {
            // When we have multiple languages we need to rewrite paths
            // on each iteration for each specific language
            if locale_map.multi {
                let locale_target = base_target.join(&lang);
                info!("lang {} -> {}", &lang, locale_target.display());

                if !locale_target.exists() {
                    fs::create_dir_all(&locale_target)?;
                }

                // Keep the target language in sync
                ctx.options.lang = lang.clone();

                // Keep the options target in sync for manifests
                ctx.options.target = locale_target.clone();

                // Rewrite the output paths and page languages
                ctx.collation
                    .rewrite(&opts, &lang, &previous_base, &locale_target)?;

                previous_base = locale_target;
            }

            //prepare(&mut ctx)?;
            //let (_, _, parse_list) = build(&mut ctx, &locales).await?;
            //finish(&mut ctx, parse_list)?;
        }

        //write_robots_file(&mut ctx)?;

        //Ok((ctx, locales))

        Ok(())
    }
    */

    async fn build(
        &self
        //ctx: &'a mut BuildContext,
        //locales: &'a Locales,
    ) -> std::result::Result<(Compiler<'_>, Parser<'_>, Vec<ParseData>), compiler::Error> {
        let parser = Parser::new(&self.context, &self.locales)?;
        let builder = Compiler::new(&self.context);

        let mut targets: Vec<PathBuf> = Vec::new();

        // FIXME: restore this!

        //if let Some(ref paths) = ctx.options.settings.paths {
            //builder.verify(paths)?;
            //for p in paths {
                //targets.push(p.clone());
            //}
        //} else {
            //targets.push(ctx.options.source.clone());
        //}

        targets.push(self.context.options.source.clone());

        let parse_list = builder.all(&parser, targets).await?;
        Ok((builder, parser, parse_list))
    }
}
