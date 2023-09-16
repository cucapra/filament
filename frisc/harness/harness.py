import calyx.builder as cb

def add_main(prog):
    main = prog.component("main")

    # instantiate cells
    pc = main.reg("pc", 32)
    pc1 = main.reg("pc1", 32)
    pc_adder = main.add("pc_adder", 32)
    add_cond = main.add("add_cond", 32)

    # make groups
    with main.group("init") as init:
        pc.write_en = 1
        pc.in_ = 0
        pc1.write_en = 1
        pc.in_ = 0

def build():
    prog = cb.Builder()
