// 1-stage pipelined floating point adder
module FP_Adder_1Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);

reg [31:0] a_reg, b_reg;

// Pipeline stage 1: Store inputs and compute result combinationally
always @(posedge clk) begin
    if (reset) begin
        a_reg <= 0;
        b_reg <= 0;
        result <= 0;
    end else begin
        a_reg <= a;
        b_reg <= b;
        result <= compute_sum(a_reg, b_reg);
    end
end

// Combinational function to compute floating point addition
function [31:0] compute_sum;
    input [31:0] num1, num2;
    reg [7:0] e1, e2, larger_exp, final_exp;
    reg [22:0] m1, m2, small_mantissa, large_mantissa, final_mantissa;
    reg [23:0] add_mantissa, norm_mantissa;
    reg s1, s2, final_sign;
    reg [4:0] shift_amt;
    reg signed [8:0] exp_adjust;
    begin
        // Extract fields
        s1 = num1[31]; s2 = num2[31];
        e1 = num1[30:23]; e2 = num2[30:23];
        m1 = num1[22:0]; m2 = num2[22:0];
        
        // Determine larger exponent and align mantissas
        if (e1 > e2) begin
            larger_exp = e1;
            large_mantissa = (e1 != 0) ? {1'b1, m1[22:1]} : m1;
            small_mantissa = (e2 != 0) ? {1'b1, m2[22:1]} : m2;
            small_mantissa = small_mantissa >> (e1 - e2);
        end else begin
            larger_exp = e2;
            large_mantissa = (e2 != 0) ? {1'b1, m2[22:1]} : m2;
            small_mantissa = (e1 != 0) ? {1'b1, m1[22:1]} : m1;
            small_mantissa = small_mantissa >> (e2 - e1);
        end
        
        // Add or subtract mantissas
        if (s1 == s2) begin
            add_mantissa = large_mantissa + small_mantissa;
        end else begin
            add_mantissa = large_mantissa - small_mantissa;
        end
        
        // Normalize result
        if (add_mantissa[23]) begin
            norm_mantissa = add_mantissa >> 1;
            exp_adjust = 1;
        end else if (add_mantissa[22]) begin
            norm_mantissa = add_mantissa;
            exp_adjust = 0;
        end else begin
            // Find first 1 bit and shift left
            shift_amt = 0;
            if (add_mantissa != 0) begin
                while (!add_mantissa[22 - shift_amt] && shift_amt < 23) 
                    shift_amt = shift_amt + 1;
                norm_mantissa = add_mantissa << shift_amt;
                exp_adjust = -shift_amt;
            end else begin
                norm_mantissa = 0;
                exp_adjust = -larger_exp;
            end
        end
        
        final_exp = larger_exp + exp_adjust[7:0];
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
        
        compute_sum = {final_sign, final_exp, final_mantissa};
    end
endfunction

endmodule