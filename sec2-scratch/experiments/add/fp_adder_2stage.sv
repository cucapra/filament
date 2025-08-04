// 2-stage pipelined floating point adder
module FP_Adder_2Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);

// Stage 1 registers
reg [7:0] e1_s1, e2_s1, larger_exp_s1;
reg [22:0] m1_s1, m2_s1, small_mantissa_s1, large_mantissa_s1;
reg s1_s1, s2_s1;
reg [4:0] shift_amt_s1;

// Stage 2 registers  
reg [23:0] add_mantissa_s2;
reg [7:0] larger_exp_s2;
reg s1_s2, s2_s2;
reg [7:0] e1_s2, e2_s2;
reg [22:0] m1_s2, m2_s2;

always @(posedge clk) begin
    if (reset) begin
        // Reset stage 1
        e1_s1 <= 0; e2_s1 <= 0; larger_exp_s1 <= 0;
        m1_s1 <= 0; m2_s1 <= 0; small_mantissa_s1 <= 0; large_mantissa_s1 <= 0;
        s1_s1 <= 0; s2_s1 <= 0; shift_amt_s1 <= 0;
        
        // Reset stage 2
        add_mantissa_s2 <= 0; larger_exp_s2 <= 0;
        s1_s2 <= 0; s2_s2 <= 0; e1_s2 <= 0; e2_s2 <= 0; m1_s2 <= 0; m2_s2 <= 0;
        result <= 0;
    end else begin
        // Stage 1: Input processing and mantissa alignment
        s1_s1 <= a[31]; s2_s1 <= b[31];
        e1_s1 <= a[30:23]; e2_s1 <= b[30:23];
        m1_s1 <= a[22:0]; m2_s1 <= b[22:0];
        
        if (a[30:23] > b[30:23]) begin
            larger_exp_s1 <= a[30:23];
            large_mantissa_s1 <= (a[30:23] != 0) ? {1'b1, a[22:1]} : a[22:0];
            small_mantissa_s1 <= ((b[30:23] != 0) ? {1'b1, b[22:1]} : b[22:0]) >> (a[30:23] - b[30:23]);
        end else begin
            larger_exp_s1 <= b[30:23];
            large_mantissa_s1 <= (b[30:23] != 0) ? {1'b1, b[22:1]} : b[22:0];
            small_mantissa_s1 <= ((a[30:23] != 0) ? {1'b1, a[22:1]} : a[22:0]) >> (b[30:23] - a[30:23]);
        end
        
        // Stage 2: Addition and result formation
        s1_s2 <= s1_s1; s2_s2 <= s2_s1;
        e1_s2 <= e1_s1; e2_s2 <= e2_s1; m1_s2 <= m1_s1; m2_s2 <= m2_s1;
        larger_exp_s2 <= larger_exp_s1;
        
        if (s1_s1 == s2_s1) begin
            add_mantissa_s2 <= large_mantissa_s1 + small_mantissa_s1;
        end else begin
            add_mantissa_s2 <= large_mantissa_s1 - small_mantissa_s1;
        end
        
        // Normalize and form result
        result <= normalize_result(add_mantissa_s2, larger_exp_s2, s1_s2, s2_s2, e1_s2, e2_s2, m1_s2, m2_s2);
    end
end

function automatic [31:0] normalize_result;
    input [23:0] add_mant;
    input [7:0] larger_exp;
    input s1, s2;
    input [7:0] e1, e2;
    input [22:0] m1, m2;
    
    reg [7:0] final_exp;
    reg [22:0] final_mantissa;
    reg final_sign;
    reg [23:0] norm_mantissa;
    integer exp_adjust;
    reg [4:0] shift_amt;
    
    begin
        // Normalize
        if (add_mant[23]) begin
            norm_mantissa = add_mant >> 1;
            exp_adjust = 1;
        end else if (add_mant[22]) begin
            norm_mantissa = add_mant;
            exp_adjust = 0;
        end else begin
            shift_amt = 0;
            if (add_mant != 0) begin
                while (!add_mant[22 - shift_amt] && shift_amt < 23) 
                    shift_amt = shift_amt + 1;
                norm_mantissa = add_mant << shift_amt;
                exp_adjust = -shift_amt;
            end else begin
                norm_mantissa = 0;
                exp_adjust = -larger_exp;
            end
        end
        
        final_exp = larger_exp + exp_adjust;
        final_mantissa = norm_mantissa[22:0];
        
        // Determine sign
        if (s1 == s2) begin
            final_sign = s1;
        end else if (e1 > e2) begin
            final_sign = s1;
        end else if (e2 > e1) begin
            final_sign = s2;
        end else if (m1 > m2) begin
            final_sign = s1;
        end else begin
            final_sign = s2;
        end
        
        normalize_result = {final_sign, final_exp, final_mantissa};
    end
endfunction

endmodule
