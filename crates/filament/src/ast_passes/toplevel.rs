use crate::ast_visitor::{Action, Visitor};
use fil_ast as ast;
use fil_utils::{self as utils, AttrCtx, Diagnostics, Error, GPosIdx};

/// Sets the proper FSM Attributes for every component
#[derive(Default)]
pub struct TopLevel {
    /// Set to true if we find a toplevel component
    has_toplevel: Option<GPosIdx>,
    /// Error reporting
    diag: Diagnostics,
}

impl Visitor for TopLevel {
    fn name() -> &'static str {
        "fsm-attributes"
    }

    fn signature(&mut self, sig: &mut ast::Signature) -> Action {
        if sig.attributes.get(utils::comp_attrs::Bool::TopLevel) == Some(&true)
        {
            if self.has_toplevel.is_some() {
                let err = Error::malformed("Multiple top-level components")
                    .add_note(self.diag.add_info(
                        "first top-level component here",
                        self.has_toplevel.unwrap(),
                    ))
                    .add_note(
                        self.diag.add_info(
                            "second top-level component here",
                            sig.attributes
                                .get_loc(utils::comp_attrs::Bool::TopLevel)
                                .unwrap(),
                        ),
                    );

                self.diag.add_error(err);
            } else {
                self.has_toplevel = Some(
                    sig.attributes
                        .get_loc(utils::comp_attrs::Bool::TopLevel)
                        .unwrap(),
                );
            }
        }

        // Stop traversal into component body
        Action::Stop
    }

    fn external(&mut self, ext: &mut ast::Extern) {
        for sig in &mut ext.comps {
            if sig.attributes.get(utils::comp_attrs::Bool::TopLevel)
                == Some(&true)
            {
                let err =
                    Error::malformed("External components cannot be top-level")
                        .add_note(
                            self.diag.add_info(
                                "toplevel attribute here",
                                sig.attributes
                                    .get_loc(utils::comp_attrs::Bool::TopLevel)
                                    .unwrap(),
                            ),
                        );

                self.diag.add_error(err);
            }
        }
    }

    fn after_traversal(mut self) -> Option<u64> {
        self.diag.report_all()
    }

    fn finish(&mut self, ast: &mut ast::Namespace) {
        // If no toplevel component was found, find the component with the name "main"
        if self.has_toplevel.is_none() {
            for comp in ast.components.iter_mut() {
                if comp.sig.name.as_ref() == "main" {
                    // Add the toplevel attribute to the component
                    comp.sig.attributes.set(
                        utils::comp_attrs::Bool::TopLevel,
                        true,
                        GPosIdx::UNKNOWN,
                    );

                    return;
                }
            }
        }
    }
}
