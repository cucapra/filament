// 4-stage pipelined floating point multiplier
module FP_Mult_4Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);

// Stage 1 registers
reg sign_s1, exc_s1;
reg [23:0] op_a_s1, op_b_s1;
reg [8:0] sum_exp_s1;

// Stage 2 registers
reg sign_s2, exc_s2;
reg [47:0] product_s2;
reg [8:0] sum_exp_s2;

// Stage 3 registers
reg sign_s3, exc_s3;
reg [47:0] product_norm_s3;
reg [8:0] exponent_s3;

// Stage 4 registers
reg sign_s4, exc_s4;
reg [22:0] final_mantissa_s4;
reg [7:0] final_exp_s4;
reg ovf_s4, unf_s4;

// Rounding logic signal
reg round;

always @(posedge clk) begin
    if (reset) begin
        // Reset all stages
        sign_s1 <= 0; exc_s1 <= 0; op_a_s1 <= 0; op_b_s1 <= 0; sum_exp_s1 <= 0;
        sign_s2 <= 0; exc_s2 <= 0; product_s2 <= 0; sum_exp_s2 <= 0;
        sign_s3 <= 0; exc_s3 <= 0; product_norm_s3 <= 0; exponent_s3 <= 0;
        sign_s4 <= 0; exc_s4 <= 0; final_mantissa_s4 <= 0; final_exp_s4 <= 0; ovf_s4 <= 0; unf_s4 <= 0;
        result <= 0; exception <= 0; overflow <= 0; underflow <= 0;
    end else begin
        // Stage 1: Input processing
        sign_s1 <= a[31] ^ b[31];
        exc_s1 <= (&a[30:23]) | (&b[30:23]);
        op_a_s1 <= (|a[30:23]) ? {1'b1, a[22:0]} : {1'b0, a[22:0]};
        op_b_s1 <= (|b[30:23]) ? {1'b1, b[22:0]} : {1'b0, b[22:0]};
        sum_exp_s1 <= a[30:23] + b[30:23];
        
        // Stage 2: Multiplication
        sign_s2 <= sign_s1;
        exc_s2 <= exc_s1;
        product_s2 <= op_a_s1 * op_b_s1;
        sum_exp_s2 <= sum_exp_s1;
        
        // Stage 3: Normalization
        sign_s3 <= sign_s2;
        exc_s3 <= exc_s2;
        
        if (product_s2[47]) begin
            product_norm_s3 <= product_s2;
            exponent_s3 <= sum_exp_s2 - 8'd127 + 1;
        end else begin
            product_norm_s3 <= product_s2 << 1;
            exponent_s3 <= sum_exp_s2 - 8'd127;
        end
        
        // Stage 4: Rounding and final result formation
        sign_s4 <= sign_s3;
        exc_s4 <= exc_s3;
        
        // Rounding
        round = |product_norm_s3[22:0];
        final_mantissa_s4 <= product_norm_s3[46:24] + (product_norm_s3[23] & round);
        final_exp_s4 <= exponent_s3[7:0];
        
        ovf_s4 <= (exponent_s3[8] & !exc_s3 & !exponent_s3[7]);
        unf_s4 <= (exponent_s3[8] & !exc_s3 & exponent_s3[7]);
        
        // Final output
        result <= exc_s4 ? {sign_s4, 31'd0} :
                  ovf_s4 ? {sign_s4, 8'hFF, 23'd0} :
                  unf_s4 ? {sign_s4, 31'd0} :
                  {sign_s4, final_exp_s4, final_mantissa_s4};
                  
        exception <= exc_s4;
        overflow <= ovf_s4;
        underflow <= unf_s4;
    end
end

endmodule
