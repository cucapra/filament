use super::{
    build_ctx::{Binding, BuildCtx},
    max_states,
    utils::{NameGenerator, INTERFACE_PORTS},
};
use crate::ir::{self, Ctx, Traversal};
use calyx_frontend as frontend;
use calyx_ir as calyx;
use calyx_utils::CalyxResult;
use std::{collections::HashSet, convert::identity, path::PathBuf, rc::Rc};

#[derive(Default)]
/// Compiles Filament directly into Calyx
/// Generates FSMs for each event (with an interface port)
pub struct Compile;

impl Compile {
    /// Compiles a port into a [calyx::PortDef].
    /// Panics if the port is not a signature port.
    fn port_def<CW, WT>(
        ctx: &ir::Context,
        comp: &ir::Component,
        port: ir::PortIdx,
        width_transform: WT, // Function thattransforms an [ir::ExprIdx] into a [CW] type
        name_gen: &NameGenerator,
    ) -> calyx::PortDef<CW>
    where
        WT: Fn(ir::ExprIdx, &ir::Component) -> CW,
    {
        let raw_port = comp.get(port);

        let ir::PortOwner::Sig { dir, .. } = &raw_port.owner else {
            unreachable!(
                "Attempting to compile non-signature port as port definition."
            )
        };

        // adds the `@data` attribute to the port
        let mut attributes = calyx::Attributes::default();
        attributes.insert(calyx::BoolAttr::Data, 1);

        calyx::PortDef {
            name: name_gen.port_name(port, ctx, comp).into(),
            width: width_transform(comp.get(port).width, comp),
            direction: match dir.reverse() {
                ir::Direction::In => calyx::Direction::Input,
                ir::Direction::Out => calyx::Direction::Output,
            },
            attributes,
        }
    }

    /// Compiles a list of ports into a [calyx::PortDef].
    fn ports<CW, WFU, WT>(
        ctx: &ir::Context,
        comp: &ir::Component,
        width_from_u64: WFU, // Function that returns a CW type from a u64
        width_transform: WT, // Function that transforms an [ir::ExprIdx] into a [CW] type
        name_gen: &NameGenerator,
    ) -> Vec<calyx::PortDef<CW>>
    where
        WFU: Fn(u64) -> CW,
        WT: Fn(ir::ExprIdx, &ir::Component) -> CW,
    {
        let mut ports: Vec<_> = comp
            // the initial list of ports.
            .ports()
            .idx_iter()
            .filter(|idx| comp.get(*idx).is_sig())
            .map(|idx| {
                Compile::port_def(ctx, comp, idx, &width_transform, name_gen)
            })
            .chain(
                // adds unannotated ports to the list of ports
                comp.unannotated_ports.iter().map(|(name, width)| {
                    calyx::PortDef {
                        name: name.as_ref().into(),
                        width: width_from_u64(*width),
                        direction: calyx::Direction::Input,
                        attributes: calyx::Attributes::default(),
                    }
                }),
            )
            .chain(
                // adds interface ports to the list of ports
                comp.events()
                    .idx_iter()
                    .filter_map(|idx| name_gen.interface_name(idx, comp))
                    .map(|name| calyx::PortDef {
                        name: name.into(),
                        width: width_from_u64(1),
                        direction: calyx::Direction::Input,
                        // adds the `@fil_event` attribute to the port
                        attributes: vec![(
                            calyx::Attribute::Unknown("fil_event".into()),
                            1,
                        )]
                        .try_into()
                        .unwrap(),
                    }),
            )
            .collect();

        let mut interface_ports =
            INTERFACE_PORTS.iter().collect::<HashSet<_>>();

        // add interface port attributes if necessary
        for pd in &mut ports {
            if let Some(pair @ ((attr, value), (name, _, dir))) =
                INTERFACE_PORTS
                    .iter()
                    .find(|(_, (n, _, _))| *n == pd.name.as_ref())
            {
                assert!(
                    dir == &pd.direction,
                    "Expected {} to be an {:?} port, got {:?} port.",
                    name,
                    dir,
                    pd.direction
                );
                // TODO: should also assert that the width of the matching port is the same as what we expect
                // We'd also need an equality function on `CW` types to do this, which we don't have at the moment
                // for [calyx::Width].

                // Removes this interface port from the list so it is not added later on.
                interface_ports.remove(pair);
                pd.attributes.insert(*attr, *value);
            }
        }

        // if this component is external, don't add new interface ports, as we must keep the signature the same
        if !comp.is_ext {
            // add remaining interface ports if not found (found ports already removed above)
            for (attr, (name, width, dir)) in interface_ports {
                ports.push(calyx::PortDef {
                    name: (*name).into(),
                    width: width_from_u64(*width),
                    direction: dir.clone(),
                    attributes: vec![*attr].try_into().unwrap(),
                });
            }
        }

        ports
    }

    /// Compiles a primitive component into a [calyx::Primitive]
    fn primitive(
        ctx: &ir::Context,
        idx: ir::CompIdx,
        name_gen: &NameGenerator,
    ) -> calyx::Primitive {
        let comp = ctx.get(idx);

        assert!(
            comp.is_ext,
            "Attempting to compile {idx} non-primitive component as primitive.",
        );

        calyx::Primitive {
            name: name_gen.comp_name(idx, ctx).into(),
            params: comp
                .params()
                .iter()
                .filter(|(_, p)| ir::ParamOwner::Sig == p.owner)
                .map(|(idx, _)| name_gen.param_name(idx, comp).into())
                .collect(),
            signature: Compile::ports(
                ctx,
                comp,
                |value| calyx::Width::Const { value },
                |idx: ir::ExprIdx, comp: &ir::Component| {
                    name_gen.expr_width(idx, comp)
                },
                name_gen,
            ),
            attributes: calyx::Attributes::default(),
            is_comb: false,
            body: None,
            latency: None,
        }
    }

    /// Compiles an [ir::Component] into a [calyx::Component]
    fn component(
        disable_slow_fsms: bool,
        ctx: &ir::Context,
        idx: ir::CompIdx,
        bind: &mut Binding,
        lib: &calyx::LibrarySignatures,
        name_gen: &NameGenerator,
    ) -> calyx::Component {
        log::debug!("Compiling component {idx}");
        let comp = ctx.get(idx);

        assert!(
            !comp.is_ext,
            "Attempting to compile primitive component as non-primitive."
        );

        let ports = Compile::ports(
            ctx,
            comp,
            identity,
            |e, comp| e.concrete(comp),
            name_gen,
        );
        let mut component = calyx::Component::new(
            name_gen.comp_name(idx, ctx),
            ports,
            false,
            false,
            None,
        );
        component.attributes.insert(calyx::BoolAttr::NoInterface, 1);

        // If this is the main component, give it a `@top_level` attribute
        if Some(idx) == ctx.entrypoint {
            log::debug!("Defining main component {idx}");
            component.attributes.insert(calyx::BoolAttr::TopLevel, 1);
        }

        let builder = calyx::Builder::new(&mut component, lib).not_generated();
        let mut buildctx = BuildCtx::new(
            ctx,
            idx,
            bind,
            disable_slow_fsms,
            name_gen,
            builder,
            lib,
        );

        // Construct all the FSMs
        for (event, states) in max_states(comp) {
            buildctx.insert_fsm(event, states);
        }

        for inst in comp.instances().idx_iter() {
            buildctx.add_instance(inst);
        }

        for inv in comp.invocations().idx_iter() {
            buildctx.add_invoke(inv);
        }

        for cmd in &comp.cmds {
            match cmd {
                ir::Command::Connect(connect) => buildctx.compile_connect(connect),
                ir::Command::ForLoop(_) => {
                    unreachable!("for loops should have been compiled away.")
                }
                ir::Command::If(_) => {
                    unreachable!("if should have been compiled away.")
                }
                ir::Command::Let(_) => {
                    unreachable!("let should have been compiled away.")
                }
                ir::Command::BundleDef(_) => {
                    unreachable!("bundle definitions should have been compiled away.")
                }
                ir::Command::Instance(_) // ignore instances and invokes as these are compiled first
                | ir::Command::Invoke(_)
                | ir::Command::Fact(_) => (),
            }
        }

        component
    }

    fn init(
        ctx: &ir::Context,
        externs: Vec<(&String, Vec<ir::CompIdx>)>,
        name_gen: &NameGenerator,
    ) -> CalyxResult<calyx::Context> {
        let mut ws = frontend::Workspace::from_compile_lib()?;
        // Add all primitives
        for (file, prims) in externs {
            for prim in prims {
                ws.lib.add_extern_primitive(
                    PathBuf::from(file),
                    Compile::primitive(ctx, prim, name_gen),
                )
            }
        }

        // define a fake main component (needed to generate the ir calyx context)
        let main =
            frontend::ast::ComponentDef::new("main", false, None, vec![]);
        ws.components.push(main);
        let mut ctx = calyx::from_ast::ast_to_ir(ws)?;
        ctx.components.retain(|c| c.name != "main");
        Ok(ctx)
    }

    /// Compiles filament into calyx
    pub fn compile(
        ctx: ir::Context,
        disable_slow_fsms: bool,
        debug: bool,
    ) -> calyx::Context {
        // Creates a map between the file name and the external components defined in that file
        let externals =
            ctx.externals.iter().map(|(k, v)| (k, v.clone())).collect();

        let name_gen = NameGenerator::new(debug);

        let mut calyx_ctx = Compile::init(&ctx, externals, &name_gen)
            .unwrap_or_else(|e| {
                panic!("Error initializing calyx context: {:?}", e);
            });

        let mut bindings = Binding::default();

        let po = Traversal::from(ctx);

        // Compile the components in post-order.
        po.apply_pre_order(|ctx, idx| {
            let comp = Compile::component(
                disable_slow_fsms,
                ctx,
                idx,
                &mut bindings,
                &calyx_ctx.lib,
                &name_gen,
            );
            bindings.insert(idx, Rc::clone(&comp.signature));
            calyx_ctx.components.push(comp);
        });

        // add the fsm components to the calyx context
        calyx_ctx.components.extend(bindings.fsm_comps.take());

        calyx_ctx
    }
}
