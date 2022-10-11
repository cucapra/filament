module Tdot (
    input wire clk,
    input wire reset,
    input wire [7:0] a0,
    input wire [7:0] a1,
    input wire [7:0] a2,
    input wire [7:0] b0,
    input wire [7:0] b1,
    input wire [7:0] b2,
    input wire [7:0] c,
    output wire [7:0] y
);

wire r;
reg [3:0] count;

// Counter
always @(posedge clk) begin
    // XXX(rachit): This is not quite right because the module cannot be reset again
    if (reset && count === 'x) begin
        count <= 0;
    end else if (count <= 3) begin
        count <= count + 1;
    end else begin
        count <= count;
    end
end

// While the counter < 3, r forwards the reset signal. Otherwise, r is 0.
assign r = (count < 3) ? reset : 1'b0;

top t (
    .clk(clk),
    .reset(r),
    .a0(a0),
    .a1(a1),
    .a2(a2),
    .b0(b0),
    .b1(b1),
    .b2(b2),
    .c(c),
    .en(1'd1),
    .y(y)
);

endmodule

module top (
    input wire clk,
    input wire reset,
    input wire [7:0] a0,
    input wire [7:0] a1,
    input wire [7:0] a2,
    input wire [7:0] b0,
    input wire [7:0] b1,
    input wire [7:0] b2,
    input wire [7:0] c,
    input wire en,
    output wire [7:0] y
);
    wire gnd;
    wire vcc;
    wire [47:0] _y;
    wire [7:0] t0;
    wire [47:0] _t0;
    wire [7:0] t1;
    wire [47:0] _t1;
    GND _gnd (
        .G(gnd)
    );
    VCC _vcc (
        .P(vcc)
    );
    DSP48E2 # (
        .ACASCREG(1),
        .ADREG(0),
        .ALUMODEREG(0),
        .AMULTSEL("A"),
        .AREG(1),
        .AUTORESET_PATDET("NO_RESET"),
        .AUTORESET_PRIORITY("RESET"),
        .A_INPUT("DIRECT"),
        .BCASCREG(1),
        .BMULTSEL("B"),
        .BREG(1),
        .B_INPUT("DIRECT"),
        .CARRYINREG(0),
        .CARRYINSELREG(0),
        .CREG(0),
        .DREG(0),
        .INMODEREG(0),
        .IS_ALUMODE_INVERTED(4'h0),
        .IS_CARRYIN_INVERTED(1'b0),
        .IS_CLK_INVERTED(1'b0),
        .IS_INMODE_INVERTED(5'h0),
        .IS_OPMODE_INVERTED(9'h0),
        .IS_RSTALLCARRYIN_INVERTED(1'b0),
        .IS_RSTALUMODE_INVERTED(1'b0),
        .IS_RSTA_INVERTED(1'b0),
        .IS_RSTB_INVERTED(1'b0),
        .IS_RSTCTRL_INVERTED(1'b0),
        .IS_RSTC_INVERTED(1'b0),
        .IS_RSTD_INVERTED(1'b0),
        .IS_RSTINMODE_INVERTED(1'b0),
        .IS_RSTM_INVERTED(1'b0),
        .IS_RSTP_INVERTED(1'b0),
        .MASK(48'h3fffffffffff),
        .MREG(1),
        .OPMODEREG(0),
        .PATTERN(48'h0),
        .PREADDINSEL("A"),
        .PREG(1),
        .RND(48'h0),
        .SEL_MASK("MASK"),
        .SEL_PATTERN("PATTERN"),
        .USE_MULT("MULTIPLY"),
        .USE_SIMD("ONE48"),
        .USE_WIDEXOR("FALSE"),
        .XORSIMD("XOR24_48_96")
    ) __y (
        .A({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, a2[7], a2[6], a2[5], a2[4], a2[3], a2[2], a2[1], a2[0]}),
        .ACIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .ACOUT(),
        .ALUMODE({gnd, gnd, gnd, gnd}),
        .B({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, b2[7], b2[6], b2[5], b2[4], b2[3], b2[2], b2[1], b2[0]}),
        .BCIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .BCOUT(),
        .C({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, t0[7], t0[6], t0[5], t0[4], t0[3], t0[2], t0[1], t0[0]}),
        .CARRYCASCIN(gnd),
        .CARRYCASCOUT(),
        .CARRYIN(gnd),
        .CARRYINSEL({gnd, gnd, gnd}),
        .CARRYOUT(),
        .CEA1(en),
        .CEA2(en),
        .CEAD(gnd),
        .CEALUMODE(gnd),
        .CEB1(en),
        .CEB2(en),
        .CEC(gnd),
        .CECARRYIN(gnd),
        .CECTRL(gnd),
        .CED(gnd),
        .CEINMODE(gnd),
        .CEM(en),
        .CEP(en),
        .CLK(clk),
        .D({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .INMODE({gnd, gnd, gnd, gnd, gnd}),
        .MULTSIGNIN(gnd),
        .MULTSIGNOUT(),
        .OPMODE({gnd, gnd, gnd, vcc, vcc, gnd, vcc, gnd, vcc}),
        .OVERFLOW(),
        .P(_y),
        .PATTERNBDETECT(),
        .PATTERNDETECT(),
        .PCIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .PCOUT(),
        .RSTA(reset),
        .RSTALLCARRYIN(reset),
        .RSTALUMODE(reset),
        .RSTB(reset),
        .RSTC(reset),
        .RSTCTRL(reset),
        .RSTD(reset),
        .RSTINMODE(reset),
        .RSTM(reset),
        .RSTP(reset),
        .UNDERFLOW(),
        .XOROUT()
    );
    assign y = _y[7:0];
    DSP48E2 # (
        .ACASCREG(1),
        .ADREG(0),
        .ALUMODEREG(0),
        .AMULTSEL("A"),
        .AREG(1),
        .AUTORESET_PATDET("NO_RESET"),
        .AUTORESET_PRIORITY("RESET"),
        .A_INPUT("DIRECT"),
        .BCASCREG(1),
        .BMULTSEL("B"),
        .BREG(1),
        .B_INPUT("DIRECT"),
        .CARRYINREG(0),
        .CARRYINSELREG(0),
        .CREG(0),
        .DREG(0),
        .INMODEREG(0),
        .IS_ALUMODE_INVERTED(4'h0),
        .IS_CARRYIN_INVERTED(1'b0),
        .IS_CLK_INVERTED(1'b0),
        .IS_INMODE_INVERTED(5'h0),
        .IS_OPMODE_INVERTED(9'h0),
        .IS_RSTALLCARRYIN_INVERTED(1'b0),
        .IS_RSTALUMODE_INVERTED(1'b0),
        .IS_RSTA_INVERTED(1'b0),
        .IS_RSTB_INVERTED(1'b0),
        .IS_RSTCTRL_INVERTED(1'b0),
        .IS_RSTC_INVERTED(1'b0),
        .IS_RSTD_INVERTED(1'b0),
        .IS_RSTINMODE_INVERTED(1'b0),
        .IS_RSTM_INVERTED(1'b0),
        .IS_RSTP_INVERTED(1'b0),
        .MASK(48'h3fffffffffff),
        .MREG(1),
        .OPMODEREG(0),
        .PATTERN(48'h0),
        .PREADDINSEL("A"),
        .PREG(1),
        .RND(48'h0),
        .SEL_MASK("MASK"),
        .SEL_PATTERN("PATTERN"),
        .USE_MULT("MULTIPLY"),
        .USE_SIMD("ONE48"),
        .USE_WIDEXOR("FALSE"),
        .XORSIMD("XOR24_48_96")
    ) __t0 (
        .A({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, a1[7], a1[6], a1[5], a1[4], a1[3], a1[2], a1[1], a1[0]}),
        .ACIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .ACOUT(),
        .ALUMODE({gnd, gnd, gnd, gnd}),
        .B({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, b1[7], b1[6], b1[5], b1[4], b1[3], b1[2], b1[1], b1[0]}),
        .BCIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .BCOUT(),
        .C({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, t1[7], t1[6], t1[5], t1[4], t1[3], t1[2], t1[1], t1[0]}),
        .CARRYCASCIN(gnd),
        .CARRYCASCOUT(),
        .CARRYIN(gnd),
        .CARRYINSEL({gnd, gnd, gnd}),
        .CARRYOUT(),
        .CEA1(en),
        .CEA2(en),
        .CEAD(gnd),
        .CEALUMODE(gnd),
        .CEB1(en),
        .CEB2(en),
        .CEC(gnd),
        .CECARRYIN(gnd),
        .CECTRL(gnd),
        .CED(gnd),
        .CEINMODE(gnd),
        .CEM(en),
        .CEP(en),
        .CLK(clk),
        .D({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .INMODE({gnd, gnd, gnd, gnd, gnd}),
        .MULTSIGNIN(gnd),
        .MULTSIGNOUT(),
        .OPMODE({gnd, gnd, gnd, vcc, vcc, gnd, vcc, gnd, vcc}),
        .OVERFLOW(),
        .P(_t0),
        .PATTERNBDETECT(),
        .PATTERNDETECT(),
        .PCIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .PCOUT(),
        .RSTA(reset),
        .RSTALLCARRYIN(reset),
        .RSTALUMODE(reset),
        .RSTB(reset),
        .RSTC(reset),
        .RSTCTRL(reset),
        .RSTD(reset),
        .RSTINMODE(reset),
        .RSTM(reset),
        .RSTP(reset),
        .UNDERFLOW(),
        .XOROUT()
    );
    assign t0 = _t0[7:0];
    DSP48E2 # (
        .ACASCREG(1),
        .ADREG(0),
        .ALUMODEREG(0),
        .AMULTSEL("A"),
        .AREG(1),
        .AUTORESET_PATDET("NO_RESET"),
        .AUTORESET_PRIORITY("RESET"),
        .A_INPUT("DIRECT"),
        .BCASCREG(1),
        .BMULTSEL("B"),
        .BREG(1),
        .B_INPUT("DIRECT"),
        .CARRYINREG(0),
        .CARRYINSELREG(0),
        .CREG(0),
        .DREG(0),
        .INMODEREG(0),
        .IS_ALUMODE_INVERTED(4'h0),
        .IS_CARRYIN_INVERTED(1'b0),
        .IS_CLK_INVERTED(1'b0),
        .IS_INMODE_INVERTED(5'h0),
        .IS_OPMODE_INVERTED(9'h0),
        .IS_RSTALLCARRYIN_INVERTED(1'b0),
        .IS_RSTALUMODE_INVERTED(1'b0),
        .IS_RSTA_INVERTED(1'b0),
        .IS_RSTB_INVERTED(1'b0),
        .IS_RSTCTRL_INVERTED(1'b0),
        .IS_RSTC_INVERTED(1'b0),
        .IS_RSTD_INVERTED(1'b0),
        .IS_RSTINMODE_INVERTED(1'b0),
        .IS_RSTM_INVERTED(1'b0),
        .IS_RSTP_INVERTED(1'b0),
        .MASK(48'h3fffffffffff),
        .MREG(1),
        .OPMODEREG(0),
        .PATTERN(48'h0),
        .PREADDINSEL("A"),
        .PREG(1),
        .RND(48'h0),
        .SEL_MASK("MASK"),
        .SEL_PATTERN("PATTERN"),
        .USE_MULT("MULTIPLY"),
        .USE_SIMD("ONE48"),
        .USE_WIDEXOR("FALSE"),
        .XORSIMD("XOR24_48_96")
    ) __t1 (
        .A({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, a0[7], a0[6], a0[5], a0[4], a0[3], a0[2], a0[1], a0[0]}),
        .ACIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .ACOUT(),
        .ALUMODE({gnd, gnd, gnd, gnd}),
        .B({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, b0[7], b0[6], b0[5], b0[4], b0[3], b0[2], b0[1], b0[0]}),
        .BCIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .BCOUT(),
        .C({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, c[7], c[6], c[5], c[4], c[3], c[2], c[1], c[0]}),
        .CARRYCASCIN(gnd),
        .CARRYCASCOUT(),
        .CARRYIN(gnd),
        .CARRYINSEL({gnd, gnd, gnd}),
        .CARRYOUT(),
        .CEA1(en),
        .CEA2(en),
        .CEAD(gnd),
        .CEALUMODE(gnd),
        .CEB1(en),
        .CEB2(en),
        .CEC(gnd),
        .CECARRYIN(gnd),
        .CECTRL(gnd),
        .CED(gnd),
        .CEINMODE(gnd),
        .CEM(en),
        .CEP(en),
        .CLK(clk),
        .D({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .INMODE({gnd, gnd, gnd, gnd, gnd}),
        .MULTSIGNIN(gnd),
        .MULTSIGNOUT(),
        .OPMODE({gnd, gnd, gnd, vcc, vcc, gnd, vcc, gnd, vcc}),
        .OVERFLOW(),
        .P(_t1),
        .PATTERNBDETECT(),
        .PATTERNDETECT(),
        .PCIN({gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd, gnd}),
        .PCOUT(),
        .RSTA(reset),
        .RSTALLCARRYIN(reset),
        .RSTALUMODE(reset),
        .RSTB(reset),
        .RSTC(reset),
        .RSTCTRL(reset),
        .RSTD(reset),
        .RSTINMODE(reset),
        .RSTM(reset),
        .RSTP(reset),
        .UNDERFLOW(),
        .XOROUT()
    );
    assign t1 = _t1[7:0];
endmodule
