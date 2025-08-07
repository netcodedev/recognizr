import onnxruntime
import sys

def inspect_model(model_path):
    """
    Loads an ONNX model and prints its input and output details.
    """
    try:
        print(f"--- Inspecting Model: {model_path} ---")
        session = onnxruntime.InferenceSession(model_path, providers=['CPUExecutionProvider'])
        
        # Print Input Details
        print("\n[INPUTS]")
        inputs = session.get_inputs()
        for i, input_tensor in enumerate(inputs):
            print(f"  Input {i}:")
            print(f"    Name: {input_tensor.name}")
            print(f"    Shape: {input_tensor.shape}")
            print(f"    Type: {input_tensor.type}")

        # Print Output Details
        print("\n[OUTPUTS]")
        outputs = session.get_outputs()
        print(f"Total number of outputs: {len(outputs)}")
        for i, output_tensor in enumerate(outputs):
            print(f"  Output {i}:")
            print(f"    Name: {output_tensor.name}")
            print(f"    Shape: {output_tensor.shape}")
            print(f"    Type: {output_tensor.type}")
        
        print("\n--- Inspection Complete ---")

    except Exception as e:
        print(f"\nAn error occurred: {e}")
        print("Please ensure the model path is correct and the file is a valid ONNX model.")


if __name__ == '__main__':
    if len(sys.argv) != 2:
        print("Usage: python inspect_model.py <path_to_model.onnx>")
        sys.exit(1)
    
    model_path = sys.argv[1]
    inspect_model(model_path)