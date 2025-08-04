// 3-stage pipelined floating point multiplier
module FP_Mult_3Stage (
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

always @(posedge clk) begin
    if (reset) begin
        // Reset all stages
        sign_s1 <= 0; exc_s1 <= 0; op_a_s1 <= 0; op_b_s1 <= 0; sum_exp_s1 <= 0;
        sign_s2 <= 0; exc_s2 <= 0; product_s2 <= 0; sum_exp_s2 <= 0;
        sign_s3 <= 0; exc_s3 <= 0; product_norm_s3 <= 0; exponent_s3 <= 0;
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
        
        // Form final result
        {result, exception, overflow, underflow} <= form_final_result(sign_s3, exc_s3, product_norm_s3, exponent_s3);
    end
end

function [34:0] form_final_result;  // {result[31:0], exception, overflow, underflow}
    input sign, exc;
    input [47:0] product_norm;
    input [8:0] exponent;
    
    reg round;
    reg [22:0] product_mantissa;
    reg ovf, unf;
    reg [31:0] res;
    
    begin
        round = |product_norm[22:0];
        product_mantissa = product_norm[46:24] + {22'b0, (product_norm[23] & round)};
        
        ovf = (exponent[8] & !exc & !exponent[7]);
        unf = (exponent[8] & !exc & exponent[7]);
        
        res = exc ? {sign, 31'd0} :
              ovf ? {sign, 8'hFF, 23'd0} :
              unf ? {sign, 31'd0} :
              {sign, exponent[7:0], product_mantissa};
              
        form_final_result = {res, exc, ovf, unf};
    end
endfunction

endmodule