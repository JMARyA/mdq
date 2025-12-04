use comrak::{
    format_commonmark,
    nodes::{AstNode, NodeValue},
    parse_document, Arena, Options,
};
use mdq::{split_frontmatter, Index};

/// Take a markdown document and a root directory and evaluate all `dataview` code blocks.
pub fn eval_dataview_blocks(md: &str, root_dir: String) -> String {
    let arena = Arena::new();

    let mut frontmatter = None;
    let md = if let Some((fm, md)) = split_frontmatter(md) {
        frontmatter = Some(fm);
        md
    } else {
        md.to_string()
    };

    let root = parse_document(&arena, &md, &Options::default());

    let index = Index::new(&root_dir, true);

    fn walk<'a>(node: &'a AstNode<'a>, arena: &'a Arena<'a>, index: &Index) {
        let mut data = node.data.borrow_mut();
        if let NodeValue::CodeBlock(ref cb) = &data.value {
            let info = cb.info.trim();

            // Dataview Codeblocks
            if info.starts_with("dataview") {
                let original = cb.literal.to_string();

                let (_, parsed) = super::query::DataviewQuery::parse(&original).unwrap();
                log::info!("Parsed this query: {parsed:?}");

                let replacement = parsed.run_on(index).to_markdown();

                data.value = NodeValue::Paragraph;
                let text = arena.alloc(comrak::nodes::AstNode::new(
                    comrak::nodes::Ast::new(NodeValue::Raw(replacement.into()), (0, 0).into())
                        .into(),
                ));
                node.append(text);
            }
        }

        for child in node.children() {
            walk(child, arena, index);
        }
    }

    let mut arena = Arena::new();
    walk(root, &mut arena, &index);

    let mut out = String::new();
    format_commonmark(root, &Options::default(), &mut out).unwrap();

    if let Some(fm) = frontmatter {
        format!("---\n{fm}\n---\n{out}")
    } else {
        out
    }
}
