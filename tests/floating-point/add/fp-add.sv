/* Name: Suhas Sunil Raut    SJSU ID: 011432980
Verilog Code For IEEE 754 Single Precision (32 bit) Floating Point Adder

This is a reduced complexity floating point adder.
   NaN, overflow, underflow and infinity values not processed

   Format for IEEE 754 SP FP:
     S    Exp     Mantissa
     31   30:23   22:0

     Note: Number = 0 for exp=0 and mantissa=0

Downloaded from: https://github.com/suhasr1991/5-Stage-Pipelined-IEEE-Single-Precision-Floating-Point-Adder-Design
     */

`timescale 1ns / 10ps

module IEEE_SP_FP_ADDER (
    input _go,
    input clk,
    input reset,
    input [31:0] Number1,
    input [31:0] Number2,
    output [31:0] Result
);

  reg [31:0] Num_shift_80, Num_shift_pipe2_80;
  reg [7:0]
      Larger_exp_80,
      Larger_exp_pipe1_80,
      Larger_exp_pipe2_80,
      Larger_exp_pipe3_80,
      Larger_exp_pipe4_80,
      Larger_exp_pipe5_80,
      Final_expo_80;
  reg [22:0]
      Small_exp_mantissa_80,
      Small_exp_mantissa_pipe2_80,
      S_exp_mantissa_pipe2_80,
      S_exp_mantissa_pipe3_80,
      Small_exp_mantissa_pipe3_80;
  reg [22:0] S_mantissa_80, L_mantissa_80;
  reg [22:0] L1_mantissa_pipe2_80, L1_mantissa_pipe3_80, Large_mantissa_80, Final_mant_80;
  reg [22:0]
      Large_mantissa_pipe2_80, Large_mantissa_pipe3_80, S_mantissa_pipe4_80, L_mantissa_pipe4_80;
  reg [23:0] Add_mant_80, Add1_mant_80, Add_mant_pipe5_80;
  reg [7:0] e1_80, e1_pipe1_80, e1_pipe2_80, e1_pipe3_80, e1_pipe4_80, e1_pipe5_80;
  reg [7:0] e2_80, e2_pipe1_80, e2_pipe2_80, e2_pipe3_80, e2_pipe4_80, e2_pipe5_80;
  reg [22:0] m1_80, m1_pipe1_80, m1_pipe2_80, m1_pipe3_80, m1_pipe4_80, m1_pipe5_80;
  reg [22:0] m2_80, m2_pipe1_80, m2_pipe2_80, m2_pipe3_80, m2_pipe4_80, m2_pipe5_80;

  reg s1_80, s2_80, Final_sign_80, s1_pipe1_80, s1_pipe2_80, s1_pipe3_80, s1_pipe4_80, s1_pipe5_80;
  reg s2_pipe1_80, s2_pipe2_80, s2_pipe3_80, s2_pipe4_80, s2_pipe5_80;
  reg [3:0] renorm_shift_80, renorm_shift_pipe5_80;
  integer signed renorm_exp_80, renorm_exp_pipe5_80;

  //reg    [3:0]  renorm_exp_80,renorm_exp_pipe5_80;
  reg [31:0] Result_80;

  assign Result = Result_80;

  always @(*) begin
    ///////////////////////// Combinational stage1 ///////////////////////////
    e1_80 = Number1[30:23];
    e2_80 = Number2[30:23];
    m1_80 = Number1[22:0];
    m2_80 = Number2[22:0];
    s1_80 = Number1[31];
    s2_80 = Number2[31];

    if (e1_80 > e2_80) begin
      Num_shift_80          = e1_80 - e2_80;  // determine number of mantissa shift
      Larger_exp_80         = e1_80;  // store higher exponent
      Small_exp_mantissa_80 = m2_80;
      Large_mantissa_80     = m1_80;
    end else begin
      Num_shift_80          = e2_80 - e1_80;
      Larger_exp_80         = e2_80;
      Small_exp_mantissa_80 = m1_80;
      Large_mantissa_80     = m2_80;
    end

    if (e1_80 == 0 | e2_80 == 0) begin
      Num_shift_80 = 0;
    end else begin
      Num_shift_80 = Num_shift_80;
    end

    //////// Combinational stage2 /////////
    //right shift mantissa of smaller exponent
    if (e1_pipe2_80 != 0) begin
      S_exp_mantissa_pipe2_80 = {1'b1, Small_exp_mantissa_pipe2_80[22:1]};
      S_exp_mantissa_pipe2_80 = (S_exp_mantissa_pipe2_80 >> Num_shift_pipe2_80);
    end else begin
      S_exp_mantissa_pipe2_80 = Small_exp_mantissa_pipe2_80;
    end
    // BUG: Uses value from previous stage.
    // CONFIRMED: Differs from no_pipe version
    // if (e2_80 != 0) begin
    if (e2_pipe2_80 != 0) begin
      L1_mantissa_pipe2_80 = {1'b1, Large_mantissa_pipe2_80[22:1]};
    end else begin
      L1_mantissa_pipe2_80 = Large_mantissa_pipe2_80;
    end

    ////////// Combinational stage3 //////
    //compare which is smaller mantissa
    if (S_exp_mantissa_pipe3_80 < L1_mantissa_pipe3_80) begin
      S_mantissa_80 = S_exp_mantissa_pipe3_80;
      L_mantissa_80 = L1_mantissa_pipe3_80;
    end else begin
      S_mantissa_80 = L1_mantissa_pipe3_80;
      L_mantissa_80 = S_exp_mantissa_pipe3_80;
    end
    //////// Combinational stage4 ////////
    //add the two mantissa's
    if (e1_pipe4_80 != 0 & e2_pipe4_80 != 0) begin
      if (s1_pipe4_80 == s2_pipe4_80) begin
        Add_mant_80 = S_mantissa_pipe4_80 + L_mantissa_pipe4_80;
      end else begin
        Add_mant_80 = L_mantissa_pipe4_80 - S_mantissa_pipe4_80;
      end
    end else begin
      Add_mant_80 = L_mantissa_pipe4_80;
    end
    //determine shifts for renormalization for mantissa and exponent
    if (Add_mant_80[23]) begin
      renorm_shift_80 = 4'd1;
      renorm_exp_80   = 4'd1;
    end else if (Add_mant_80[22]) begin
      renorm_shift_80 = 4'd2;
      renorm_exp_80   = 0;
    end else if (Add_mant_80[21]) begin
      renorm_shift_80 = 4'd3;
      renorm_exp_80   = -1;
    end else if (Add_mant_80[20]) begin
      renorm_shift_80 = 4'd4;
      renorm_exp_80   = -2;
    end else if (Add_mant_80[19]) begin
      renorm_shift_80 = 4'd5;
      renorm_exp_80   = -3;
    end else begin
      renorm_shift_80 = 0;
      // BUG: if the first 1 bit is after the 5th bit, normalization isn't performed.
      // BUG: if there are no zeros in the mantissa, the exponent should be set to zero.
      // CONFIRMED: left=3212836864, right=1065353216
      renorm_exp_80   = -Larger_exp_pipe4_80;
    end
    ////// Combinational stage5 //////
    //Shift the mantissa as required; re-normalize exp; determine sign

    // BUG: uses value from the previous stage.
    // CONFIRMED: left=1197990852, right=1203616721
    // Final_expo_80 = Larger_exp_pipe5_80 + renorm_exp_80;
    Final_expo_80 = Larger_exp_pipe5_80 + renorm_exp_pipe5_80;

    if (renorm_shift_pipe5_80 != 0) begin
      Add1_mant_80 = Add_mant_pipe5_80 << renorm_shift_pipe5_80;
    end else begin
      Add1_mant_80 = Add_mant_pipe5_80;
    end
    Final_mant_80 = Add1_mant_80[23:1];

    if (s1_pipe5_80 == s2_pipe5_80) begin
      Final_sign_80 = s1_pipe5_80;
    end else if (e1_pipe5_80 > e2_pipe5_80) begin
      Final_sign_80 = s1_pipe5_80;
      // BUG: uses value from the first pipeline stage.
      // CONFIRMED: left=3348281187, right=1203616721
      // end else if (e2_80 > e1_80) begin
    end else if (e2_pipe5_80 > e1_pipe5_80) begin
      Final_sign_80 = s2_pipe5_80;
    end else if (m1_pipe5_80 > m2_pipe5_80) begin
      Final_sign_80 = s1_pipe5_80;
    end else begin
      Final_sign_80 = s2_pipe5_80;
    end

    Result_80 = {Final_sign_80, Final_expo_80, Final_mant_80};
  end

  always @(posedge clk) begin
    if (reset) begin  //reset all reg at reset signal
      s1_pipe2_80                 <= 0;
      s2_pipe2_80                 <= 0;
      e1_pipe2_80                 <= 0;
      e2_pipe2_80                 <= 0;
      m1_pipe2_80                 <= 0;
      m2_pipe2_80                 <= 0;
      Larger_exp_pipe2_80         <= 0;
      //stage2
      Small_exp_mantissa_pipe2_80 <= 0;
      Large_mantissa_pipe2_80     <= 0;
      Num_shift_pipe2_80          <= 0;
      s1_pipe3_80                 <= 0;
      s2_pipe3_80                 <= 0;
      e1_pipe3_80                 <= 0;
      e2_pipe3_80                 <= 0;
      m1_pipe3_80                 <= 0;
      m2_pipe3_80                 <= 0;
      Larger_exp_pipe3_80         <= 0;
      s1_pipe4_80                 <= 0;
      s2_pipe4_80                 <= 0;
      e1_pipe4_80                 <= 0;
      e2_pipe4_80                 <= 0;
      m1_pipe4_80                 <= 0;
      m2_pipe4_80                 <= 0;
      Larger_exp_pipe4_80         <= 0;
      s1_pipe5_80                 <= 0;
      s2_pipe5_80                 <= 0;
      e1_pipe5_80                 <= 0;
      e2_pipe5_80                 <= 0;
      m1_pipe5_80                 <= 0;
      m2_pipe5_80                 <= 0;
      Larger_exp_pipe5_80         <= 0;
      //stage3
      S_exp_mantissa_pipe3_80     <= 0;
      L1_mantissa_pipe3_80        <= 0;
      //stage4
      S_mantissa_pipe4_80         <= 0;
      L_mantissa_pipe4_80         <= 0;
      //stage5
      Add_mant_pipe5_80           <= 0;
      renorm_shift_pipe5_80       <= 0;
    end else begin
      ///////////////////////////////PIPELINE STAGES and VARIABLES/////////////////
      //propogate pipelined variables to next stages
      s1_pipe2_80                 <= s1_80;
      s2_pipe2_80                 <= s2_80;
      e1_pipe2_80                 <= e1_80;
      e2_pipe2_80                 <= e2_80;
      m1_pipe2_80                 <= m1_80;
      m2_pipe2_80                 <= m2_80;
      Larger_exp_pipe2_80         <= Larger_exp_80;
      //stage2
      Small_exp_mantissa_pipe2_80 <= Small_exp_mantissa_80;
      Large_mantissa_pipe2_80     <= Large_mantissa_80;
      Num_shift_pipe2_80          <= Num_shift_80;
      s1_pipe3_80                 <= s1_pipe2_80;
      s2_pipe3_80                 <= s2_pipe2_80;
      e1_pipe3_80                 <= e1_pipe2_80;
      e2_pipe3_80                 <= e2_pipe2_80;
      m1_pipe3_80                 <= m1_pipe2_80;
      m2_pipe3_80                 <= m2_pipe2_80;
      Larger_exp_pipe3_80         <= Larger_exp_pipe2_80;
      s1_pipe4_80                 <= s1_pipe3_80;
      s2_pipe4_80                 <= s2_pipe3_80;
      e1_pipe4_80                 <= e1_pipe3_80;
      e2_pipe4_80                 <= e2_pipe3_80;
      m1_pipe4_80                 <= m1_pipe3_80;
      m2_pipe4_80                 <= m2_pipe3_80;
      Larger_exp_pipe4_80         <= Larger_exp_pipe3_80;
      s1_pipe5_80                 <= s1_pipe4_80;
      s2_pipe5_80                 <= s2_pipe4_80;
      e1_pipe5_80                 <= e1_pipe4_80;
      e2_pipe5_80                 <= e2_pipe4_80;
      m1_pipe5_80                 <= m1_pipe4_80;
      m2_pipe5_80                 <= m2_pipe4_80;
      Larger_exp_pipe5_80         <= Larger_exp_pipe4_80;
      //stage3
      S_exp_mantissa_pipe3_80     <= S_exp_mantissa_pipe2_80;
      L1_mantissa_pipe3_80        <= L1_mantissa_pipe2_80;
      //stage4
      S_mantissa_pipe4_80         <= S_mantissa_80;
      L_mantissa_pipe4_80         <= L_mantissa_80;
      //stage5
      Add_mant_pipe5_80           <= Add_mant_80;
      renorm_shift_pipe5_80       <= renorm_shift_80;
      renorm_exp_pipe5_80         <= renorm_exp_80;
    end
  end

endmodule
