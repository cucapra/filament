use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    path::PathBuf,
    rc::Rc,
};

use crate::ir::{self, Ctx, Traversal};
use calyx_frontend as frontend;
use calyx_ir as calyx;
use calyx_utils::CalyxResult;

use super::{
    build_ctx::{Binding, BuildCtx},
    utils::INTERFACE_PORTS,
};

/// Compiles Filament into Calyx
/// Generates FSMs for each event
#[derive(Default)]
pub struct Compile;

impl Compile {
    fn port_def<CW, WT>(
        ctx: &ir::Context,
        comp: &ir::Component,
        port: ir::PortIdx,
        width_transform: WT,
    ) -> Option<calyx::PortDef<CW>>
    where
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let raw_port = comp.get(port);

        if let ir::PortOwner::Sig { dir, .. } = &raw_port.owner {
            let name = port.name(ctx, comp);
            let mut attributes = calyx::Attributes::default();
            attributes.insert(calyx::BoolAttr::Data, 1);
            Some(calyx::PortDef {
                name: name.into(),
                width: width_transform(comp, comp.get(port).width),
                direction: calyx::Direction::from(dir).reverse(),
                attributes,
            })
        } else {
            None
        }
    }

    fn ports<CW, WFU, WT>(
        ctx: &ir::Context,
        comp: &ir::Component,
        width_from_u64: WFU, // Function that returns a CW type from a u64
        width_transform: WT,
        is_comp: bool,
    ) -> Vec<calyx::PortDef<CW>>
    where
        WFU: Fn(u64) -> CW,
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let mut ports: Vec<_> = comp
            // the initial list of ports.
            .ports()
            .idx_iter()
            .filter_map(|port| {
                Compile::port_def(ctx, comp, port, &width_transform)
            })
            .chain(
                // adds unannotated ports to the list of ports
                comp.unannotated_ports().into_iter().map(|(name, width)| {
                    calyx::PortDef {
                        name: name.as_ref().into(),
                        width: width_from_u64(width),
                        direction: calyx::Direction::Input,
                        attributes: calyx::Attributes::default(),
                    }
                }),
            )
            .chain(
                // adds interface ports to the list of ports
                comp.events()
                    .idx_iter()
                    .filter_map(|idx| idx.interface_name(comp))
                    .into_iter()
                    .map(|name| calyx::PortDef {
                        name: name.into(),
                        width: width_from_u64(1),
                        direction: calyx::Direction::Input,
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
                interface_ports.remove(pair);
                pd.attributes.insert(*attr, *value);
            }
        }

        if is_comp {
            // add remaining interface ports if not found
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

    fn primitive(ctx: &ir::Context, idx: ir::CompIdx) -> calyx::Primitive {
        let comp = ctx.get(idx);
        calyx::Primitive {
            name: idx.name(ctx).into(),
            params: comp
                .params()
                .iter()
                .filter(|(_, p)| ir::ParamOwner::Sig == p.owner)
                .map(|(idx, _)| idx.name(comp).into())
                .collect(),
            signature: Compile::ports(
                ctx,
                comp,
                |value| calyx::Width::Const { value },
                |comp, expr| expr.as_width(comp),
                false,
            ),
            attributes: calyx::Attributes::default(),
            is_comb: false,
            body: None,
            latency: None,
        }
    }

    /// Gets the maximum states for each event with an interface port in the component
    fn max_states(comp: &ir::Component) -> HashMap<ir::EventIdx, u64> {
        let mut event_map = HashMap::new();

        comp.times().iter().for_each(|(_, time)| {
            let nv = time.offset.as_u64(comp);
            if nv > *event_map.get(&time.event).unwrap_or(&0) {
                event_map.insert(time.event, nv);
            }
        });

        event_map
    }

    fn component(
        ctx: &ir::Context,
        idx: ir::CompIdx,
        bind: &mut Binding,
        lib: &calyx::LibrarySignatures,
    ) -> calyx::Component {
        log::debug!("Compiling component {idx}");
        let comp = ctx.get(idx);
        let ports = Compile::ports(
            ctx,
            comp,
            identity,
            |comp, expr| expr.as_u64(comp),
            true,
        );
        let mut component =
            calyx::Component::new(idx.name(ctx), ports, false, false, None);
        component.attributes.insert(calyx::BoolAttr::NoInterface, 1);
        // main component
        if Some(idx) == ctx.entrypoint {
            log::debug!("Defining main component {idx}");
            component.attributes.insert(calyx::BoolAttr::TopLevel, 1);
        }

        let builder = calyx::Builder::new(&mut component, lib).not_generated();
        let mut ctx = BuildCtx::new(ctx, idx, bind, builder, lib);

        // Construct all the FSMs
        for (event, states) in Compile::max_states(comp) {
            ctx.insert_fsm(event, states);
        }

        let mut cons = vec![];

        for cmd in &comp.cmds {
            match cmd {
                ir::Command::Instance(idx) => ctx.add_instance(*idx),
                ir::Command::Invoke(idx) => ctx.add_invoke(*idx),
                ir::Command::Connect(connect) => cons.push(connect), // connects will be compiled later
                ir::Command::ForLoop(_) => {
                    unreachable!("For loops should have been compiled away.")
                }
                ir::Command::If(_) => {
                    unreachable!("Ifs should have been compiled away.")
                }
                ir::Command::Fact(_) => (),
            }
        }

        cons.into_iter().for_each(|con| ctx.compile_connect(con));
        component
    }

    fn init(
        ctx: &ir::Context,
        externs: Vec<(&String, Vec<ir::CompIdx>)>,
    ) -> CalyxResult<calyx::Context> {
        let mut ws = frontend::Workspace::from_compile_lib()?;
        // Add externals
        ws.externs.extend(externs.into_iter().map(|(file, comps)| {
            (
                Some(PathBuf::from(file)),
                comps
                    .into_iter()
                    .map(|idx| Compile::primitive(ctx, idx))
                    .collect(),
            )
        }));

        // define a fake main component
        let main =
            frontend::ast::ComponentDef::new("main", false, None, vec![]);
        ws.components.push(main);
        let mut ctx = calyx::from_ast::ast_to_ir(ws)?;
        ctx.components.retain(|c| c.name != "main");
        Ok(ctx)
    }

    pub fn compile(ctx: ir::Context) {
        let externals =
            ctx.externals.iter().map(|(k, v)| (k, v.clone())).collect();
        let mut calyx_ctx =
            Compile::init(&ctx, externals).unwrap_or_else(|e| {
                panic!("Error initializing calyx context: {:?}", e);
            });

        let mut bindings = Binding::default();

        let po = Traversal::from(ctx);

        po.apply_pre_order(|ctx, idx| {
            let comp =
                Compile::component(ctx, idx, &mut bindings, &calyx_ctx.lib);
            bindings.insert(idx, Rc::clone(&comp.signature));
            calyx_ctx.components.push(comp);
        });

        calyx_ctx
            .components
            .extend(bindings.fsm_comps.into_values());

        let mut out = &mut std::io::stdout();
        calyx::Printer::write_context(&calyx_ctx, false, &mut out).unwrap();
    }
}