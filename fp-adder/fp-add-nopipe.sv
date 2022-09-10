`timescale 1ns / 10ps

module IEEE_SP_FP_ADDER_NOPIPE (
    input _go,
    input clk,
    input reset,
    input [31:0] Number1,
    input [31:0] Number2,
    output [31:0] Result
);

  reg [31:0] Num_shift_80;
  reg [7:0] Larger_exp_80, Final_expo_80;
  reg [22:0] Small_exp_mantissa_80, S_mantissa_80, L_mantissa_80, Large_mantissa_80, Final_mant_80;
  reg [23:0] Add_mant_80, Add1_mant_80;
  reg [7:0] e1_80, e2_80;
  reg [22:0] m1_80, m2_80;
  reg s1_80, s2_80, Final_sign_80;
  reg    [3:0]  renorm_shift_80;
  integer signed   renorm_exp_80;
  reg    [31:0] Result_80;

  assign Result = Result_80;

  always_comb begin
    //stage 1
    e1_80 = Number1[30:23];
    e2_80 = Number2[30:23];
    m1_80 = Number1[22:0];
    m2_80 = Number2[22:0];
    s1_80 = Number1[31];
    s2_80 = Number2[31];

    if (e1_80 > e2_80) begin
      Num_shift_80          = e1_80 - e2_80;  // number of mantissa shift
      Larger_exp_80         = e1_80;  // store lower exponent
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



    //stage 2
    //if check both for normalization then append 1 and shift
    if (e1_80 != 0) begin
      Small_exp_mantissa_80 = {1'b1, Small_exp_mantissa_80[22:1]};
      Small_exp_mantissa_80 = (Small_exp_mantissa_80 >> Num_shift_80);
    end else begin
      Small_exp_mantissa_80 = Small_exp_mantissa_80;
    end

    if (e2_80 != 0) begin
      Large_mantissa_80 = {1'b1, Large_mantissa_80[22:1]};
    end else begin
      Large_mantissa_80 = Large_mantissa_80;
    end

    //else do what to do for denorm field


    //stage 3
    //check if exponent are equal
    if (Small_exp_mantissa_80 < Large_mantissa_80) begin
      //Small_exp_mantissa_80 = ((~ Small_exp_mantissa_80 ) + 1'b1);
      //$display("what small_exp:%b",Small_exp_mantissa_80);
      S_mantissa_80 = Small_exp_mantissa_80;
      L_mantissa_80 = Large_mantissa_80;
    end else begin
      //Large_mantissa_80 = ((~ Large_mantissa_80 ) + 1'b1);
      //$display("what large_exp:%b",Large_mantissa_80);

      S_mantissa_80 = Large_mantissa_80;
      L_mantissa_80 = Small_exp_mantissa_80;
    end
    //stage 4
    //add the two mantissa's

    if (e1_80 != 0 & e2_80 != 0) begin
      if (s1_80 == s2_80) begin
        Add_mant_80 = S_mantissa_80 + L_mantissa_80;
      end else begin
        Add_mant_80 = L_mantissa_80 - S_mantissa_80;
      end
    end else begin
      Add_mant_80 = L_mantissa_80;
    end

    //renormalization for mantissa and exponent
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
      renorm_exp_80 = 0;
    end

    //stage 5
    // if e1==e2, no shift for exp
    Final_expo_80 = Larger_exp_80 + renorm_exp_80;

    if (renorm_shift_80 != 0) begin
      Add1_mant_80 = Add_mant_80 << renorm_shift_80;
    end else begin
      Add1_mant_80 = Add_mant_80;
    end
    Final_mant_80 = Add1_mant_80[23:1];


    if (s1_80 == s2_80) begin
      Final_sign_80 = s1_80;
    end else if (e1_80 > e2_80) begin
      Final_sign_80 = s1_80;
    end else if (e2_80 > e1_80) begin
      Final_sign_80 = s2_80;
    end else if (m1_80 > m2_80) begin
      Final_sign_80 = s1_80;
    end else begin
      Final_sign_80 = s2_80;
    end

    Result_80 = {Final_sign_80, Final_expo_80, Final_mant_80};

  end

endmodule
