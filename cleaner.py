import os

root = "./state"
for node in os.listdir(root):
    # Delete files in 'files' folder
    files_path = os.path.join(root, node, "files")
    for file in os.listdir(files_path):
        os.remove(os.path.join(files_path, file))

    # Clear files written in file_state.json
    file_state_json_path = os.path.join(root, node, "file_state.json")
    with open(file_state_json_path, "w") as file_state_json:
        file_state_json.write(r"{}")
