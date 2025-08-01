import FIFO::*;
import FIFOF::*;
import List::*;
import Randomizable::*;

interface Compute#(numeric type width);
  method Action put(Bit#(width) l, Bit#(width) r);
  method ActionValue#(Bit#(width)) get();
endinterface

module mkCompute#(
  numeric width,
  Integer stages,
  String name,
  function Bit#(width) computeFn(Bit#(width) l, Bit#(width) r)
)(Compute#(width));
  List#(Array#(Reg#(Bit#(width)))) regs <- replicateM(stages, mkCReg(2, 0));
  List#(Array#(Reg#(Bool))) valid <- replicateM(stages, mkCReg(2, False));
  FIFOF#(Bit#(width)) fifo_in <- mkSizedFIFOF(stages);
  FIFOF#(Bit#(width)) fifo_out <- mkSizedFIFOF(stages);

  rule push_first if (fifo_in.notEmpty);
    let v = fifo_in.first; fifo_in.deq;
    regs[0][0] <= v;
    valid[0][0] <= True;
  endrule

  rule push_nothing if (!fifo_in.notEmpty);
    regs[0][0] <= ?;
    valid[0][0] <= False;
  endrule

  rule shift if (fifo_out.notFull);
    for (Integer i = 0; i < stages - 1; i = i + 1) begin
      regs[i+1][0] <= regs[i][0];
      valid[i+1][0] <= valid[i][0];
    end
  endrule

  rule write_out if (fifo_out.notFull && valid[stages-1][1]);
    fifo_out.enq(regs[stages - 1][1]);
  endrule

  rule visualize_valids;
    // Concat all valids into a single bit vector for debugging
    $write("%s: ", name);
    for (Integer i = 0; i < stages; i = i + 1) begin
      Bit#(1) validBit = valid[i][1] ? 1 : 0;
      $write("%0b", validBit);
    end
    $display("");
  endrule

  method Action put(Bit#(width) l, Bit#(width) r);
    fifo_in.enq(computeFn(l, r));
  endmethod

  method ActionValue#(Bit#(width)) get();
    let out = fifo_out.first; fifo_out.deq();
    return out;
  endmethod
endmodule

interface ALU#(numeric type width);
  method Action put(Bool op, Bit#(width) l, Bit#(width) r);
  method ActionValue#(Bit#(width)) get();
endinterface

module mkALU#(numeric width)(ALU#(width));
  function Bit#(width) add(Bit#(width) l, Bit#(width) r);
    return l + r;
  endfunction

  function Bit#(width) mul(Bit#(width) l, Bit#(width) r);
    return l * r;
  endfunction

  Integer add_stages = 2;  // Number of stages for adder
  Integer mul_stages = 3;  // Number of stages for multiplier
  Integer max_stages = max(add_stages, mul_stages);

  // Track the next computation to pull from.
  FIFOF#(Bool) next_op <- mkSizedFIFOF(5);
  Compute#(width) adder <- mkCompute(width, add_stages, "add", add);
  Compute#(width) mult <- mkCompute(width, mul_stages, "mul", mul);
  FIFO#(Bit#(width)) fifo_out <- mkFIFO;

  rule pull_adder (next_op.notEmpty && next_op.first);
    let out <- adder.get();
    next_op.deq();
    fifo_out.enq(out);
  endrule

  rule pull_mult (next_op.notEmpty && !next_op.first);
    let out <- mult.get();
    next_op.deq();
    fifo_out.enq(out);
  endrule

  method Action put(Bool op, Bit#(width) l, Bit#(width) r);
    next_op.enq(op);
    if (op)
      adder.put(l, r);
    else
      mult.put(l, r);
  endmethod

  method ActionValue#(Bit#(width)) get();
    let out = fifo_out.first; fifo_out.deq();
    return out;
  endmethod
endmodule

module top(Empty);
  Reg#(Bit#(8)) idx <- mkReg(0);
  Reg#(Bit#(8)) checked <- mkReg(0);
  Reg#(Bit#(32)) cycles <- mkReg(0);
  ALU#(32) alu <- mkALU(32);
  FIFO#(Bit#(32)) expected <- mkSizedFIFO(10);

  Randomize#(Bit#(32)) randL <- mkConstrainedRandomizer(1, 100);
  Randomize#(Bit#(32)) randR <- mkConstrainedRandomizer(1, 100);
  Randomize#(Bool) randOp <- mkGenericRandomizer;

  rule count;
    cycles <= cycles + 1;
    // $display("====Cycle %0d====", cycles);
  endrule

  rule init (cycles == 0);
    randL.cntrl.init();
    randR.cntrl.init();
    randOp.cntrl.init();
  endrule

  rule feed (idx < 10);
    Bit#(32) l <- randL.next;
    Bit#(32) r <- randR.next;
    Bool op <- randOp.next;

    alu.put(op, l, r);
    if (op)
      expected.enq(l + r);
    else
      expected.enq(l * r);
    idx <= idx + 1;
    $display("%0d: %0s, %0d, %0d", cycles, op ? "add" : "mul", l, r);
  endrule  (* fire_when_enabled *)
  rule check;
    let out <- alu.get();
    let gold = expected.first; expected.deq();
    checked <= checked + 1;
    if (gold != out) begin
      $display("%0d: %0d == %0d", cycles, gold, out);
    end
  endrule

  rule complete (checked == 10);
    $display("Simulation completed in %0d cycles", cycles - 1);
    $finish;
  endrule
endmodule