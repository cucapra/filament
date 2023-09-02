use crate::ir::{self, Ctx};
use std::fmt::Write;

/// A context capable of displaying a value.
pub trait DisplayCtx<T> {
    /// Write the value into a buffer
    fn write(&self, val: T, f: &mut impl Write) -> std::fmt::Result;

    /// Display the value
    fn display(&self, val: T) -> String {
        let mut s = String::new();
        self.write(val, &mut s).unwrap();
        s
    }
}

impl DisplayCtx<ir::CompIdx> for ir::Context {
    fn write(&self, val: ir::CompIdx, f: &mut impl Write) -> std::fmt::Result {
        let comp = self.get(val);
        if let Some(ext_info) = &comp.src_info {
            write!(f, "{}", ext_info.name)
        } else {
            write!(f, "comp{}", val.get())
        }
    }
}

impl DisplayCtx<ir::TimeIdx> for ir::Component {
    fn write(&self, val: ir::TimeIdx, f: &mut impl Write) -> std::fmt::Result {
        let &ir::Time { event, offset } = self.get(val);
        if offset.is_const(self, 0) {
            self.write(event, f)
        } else {
            self.write(event, f)?;
            write!(f, "+")?;
            self.write(offset, f)
        }
    }
}

impl DisplayCtx<ir::EventIdx> for ir::Component {
    fn write(&self, idx: ir::EventIdx, f: &mut impl Write) -> std::fmt::Result {
        if log::log_enabled!(log::Level::Debug) {
            return write!(f, "'{idx}");
        }
        let ev = self.get(idx);
        if let Some(ev) = self.get(ev.info).as_event() {
            write!(f, "'{}", ev.name)
        } else {
            write!(f, "'{idx}")
        }
    }
}

impl DisplayCtx<ir::ParamIdx> for ir::Component {
    fn write(&self, idx: ir::ParamIdx, f: &mut impl Write) -> std::fmt::Result {
        if log::log_enabled!(log::Level::Debug) {
            return write!(f, "{idx}");
        }
        let param: &ir::Param = self.get(idx);
        let info = self.get(param.info).as_param();
        let name = info.map_or(format!("{idx}"), |p| format!("{}", p.name));
        match param.owner {
            ir::ParamOwner::Instance(inst) => {
                let inst = self.get(inst);
                let inst_name = self
                    .get(inst.info)
                    .as_instance()
                    .map_or(format!("{idx}"), |inst| format!("{}", inst.name));
                write!(f, "{inst_name}::{name}")
            }
            _ => write!(f, "{name}"),
        }
    }
}

impl DisplayCtx<ir::InvIdx> for ir::Component {
    fn write(&self, idx: ir::InvIdx, f: &mut impl Write) -> std::fmt::Result {
        if log::log_enabled!(log::Level::Debug) {
            return write!(f, "{idx}");
        }

        let inv = self.get(idx);
        if let Some(inv) = self.get(inv.info).as_invoke() {
            write!(f, "{}", inv.name)
        } else {
            write!(f, "{idx}")
        }
    }
}

impl DisplayCtx<ir::InstIdx> for ir::Component {
    fn write(&self, idx: ir::InstIdx, f: &mut impl Write) -> std::fmt::Result {
        if log::log_enabled!(log::Level::Debug) {
            return write!(f, "{idx}");
        }

        let inst = self.get(idx);
        if let Some(inst) = self.get(inst.info).as_instance() {
            write!(f, "{}", inst.name)
        } else {
            write!(f, "{idx}")
        }
    }
}

impl DisplayCtx<ir::PortIdx> for ir::Component {
    fn write(&self, idx: ir::PortIdx, f: &mut impl Write) -> std::fmt::Result {
        let port = self.get(idx);
        let name = self
            .get(port.info)
            .as_port()
            .map_or(format!("{idx}"), |p| format!("{}", p.name));
        match port.owner {
            ir::PortOwner::Local | ir::PortOwner::Sig { .. } => {
                write!(f, "{name}")
            }
            ir::PortOwner::Inv { inv, .. } => {
                self.write(inv, f)?;
                write!(f, ".{name}")
            }
        }
    }
}

impl<'a> DisplayCtx<&'a ir::TimeSub> for ir::Component {
    fn write(
        &self,
        ts: &'a ir::TimeSub,
        f: &mut impl Write,
    ) -> std::fmt::Result {
        match ts {
            ir::TimeSub::Unit(e) => self.write(*e, f),
            ir::TimeSub::Sym { l, r } => {
                write!(f, "|")?;
                self.write(*l, f)?;
                write!(f, " - ")?;
                self.write(*r, f)?;
                write!(f, "|")
            }
        }
    }
}

impl<'a> DisplayCtx<&'a ir::Range> for ir::Component {
    fn write(
        &self,
        val: &'a ir::Range,
        f: &mut impl Write,
    ) -> std::fmt::Result {
        write!(f, "[")?;
        self.write(val.start, f)?;
        write!(f, ", ")?;
        self.write(val.end, f)?;
        write!(f, "]")
    }
}

impl<'a> DisplayCtx<&'a ir::Liveness> for ir::Component {
    fn write(&self, l: &ir::Liveness, f: &mut impl Write) -> std::fmt::Result {
        let ir::Liveness { idx, len, range } = l;
        write!(
            f,
            "for<{}: {}> {}",
            self.display(*idx),
            self.display(*len),
            self.display(range)
        )
    }
}

impl<'a> DisplayCtx<&'a ir::Access> for ir::Component {
    fn write(&self, a: &ir::Access, f: &mut impl Write) -> std::fmt::Result {
        let &ir::Access { port, start, end } = a;
        self.write(port, f)?;
        if a.is_port(self) {
            write!(f, "[{}]", self.display(start))
        } else {
            write!(f, "[{}..{})", self.display(start), self.display(end))
        }
    }
}

impl<'a> DisplayCtx<&'a ir::Connect> for ir::Component {
    fn write(&self, c: &ir::Connect, f: &mut impl Write) -> std::fmt::Result {
        let ir::Connect { src, dst, .. } = c;
        self.write(src, f)?;
        write!(f, " = ")?;
        self.write(dst, f)
    }
}
