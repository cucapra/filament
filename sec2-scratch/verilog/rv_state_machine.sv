// Reusable ready-valid state machine for latency-insensitive modules
module RV_StateMachine #(
    parameter PIPELINE_STAGES = 1
) (
    input logic valid_in,
    input logic valid_out,
    input logic ready_in,
    output logic ready_out,
    input logic [$clog2(PIPELINE_STAGES+1)-1:0] stage_counter,
    input logic [1:0] current_state,
    output logic [1:0] next_state
);
    typedef enum logic [1:0] {
        IDLE    = 2'b00,
        COMPUTE = 2'b01,
        VALID   = 2'b10
    } state_t;

    always_comb begin
        next_state = current_state;
        ready_out = 1'b0;
        case (current_state)
            IDLE: begin
                ready_out = 1'b1;
                if (valid_in && ready_out) begin
                    next_state = COMPUTE;
                end
            end
            COMPUTE: begin
                ready_out = 1'b0;
                if (stage_counter >= PIPELINE_STAGES - 1) begin
                    next_state = VALID;
                end
            end
            VALID: begin
                ready_out = 1'b0;
                if (valid_out && ready_in) begin
                    next_state = IDLE;
                end
            end
            default: begin
                next_state = IDLE;
            end
        endcase
    end
endmodule
