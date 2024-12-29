import os
import re
import json
import sys
import yaml
from extract_raw_data import metrics_to_extract, create_results_dict, get_bandwidth_bytes, get_proxy_request_or_start_time, get_incoming_messages, get_histogram_metrics


metrics_to_extract = [
    "loopix_bandwidth_bytes",
    "loopix_number_of_proxy_requests",
    "loopix_start_time_seconds",
    "loopix_incoming_messages",
    "loopix_end_to_end_latency_seconds",
    "loopix_encryption_latency_milliseconds",
    "loopix_client_delay_milliseconds",
    "loopix_decryption_latency_milliseconds",
    "loopix_mixnode_delay_milliseconds",
    "loopix_provider_delay_milliseconds",
]

def simulation_ran_successfully(data_dir, time_index, retrieve_index):
    dir = f"{data_dir}/"
    metrics_file = os.path.join(dir, f"metrics_{time_index}-{retrieve_index}_node-1.txt")

    if not os.path.exists(metrics_file):
        return False

    with open(metrics_file, 'r') as f:
        content = f.read()

    if "loopix_number_of_proxy_requests" in content:
        return True
    else:
        return False


def get_metrics_data(data_dir, path_length, results, time_index, retrieve_index):

    if not simulation_ran_successfully(data_dir, time_index, retrieve_index):
        print(f"Skipping run {time_index}-{retrieve_index}, no end-to-end latency data found")
        return False

    create_results_dict(results, metrics_to_extract)

    for i in range(path_length*path_length + path_length * 2):
        print(f"Getting metrics data for node-{i}")
        dir = f"{data_dir}"
        metrics_file = os.path.join(dir, f"metrics_{time_index}-{retrieve_index}_node-{i}.txt")
        
        if os.path.exists(metrics_file):
            with open(metrics_file, 'r') as f:
                content = f.read()
            
            for metric in metrics_to_extract:
                if metric == "loopix_bandwidth_bytes":
                    get_bandwidth_bytes(results, content, metric, i)

                elif metric == "loopix_number_of_proxy_requests" or metric == "loopix_start_time_seconds":
                    get_proxy_request_or_start_time(results, content, metric, i)
                
                elif metric == "loopix_incoming_messages":
                    get_incoming_messages(results, content, metric, i)

                else:
                    get_histogram_metrics(results, content, metric, i)

    print(results)
    return True
          
def main():
    if len(sys.argv) != 3:
        print("Usage: python extract_raw_data.py <data_dir> <path_length>")
        sys.exit(1)

    base_path = sys.argv[1]
    path_length = int(sys.argv[2])

    data_dir = os.path.join(base_path, "raw_data")

    # get n times
    times_suffix = "-3_node-0.txt"
    files = os.listdir(data_dir)
    
    n_times = sum(1 for file in files
                if file.endswith(times_suffix) and os.path.isfile(os.path.join(data_dir, file)))
    print("n_times: ", n_times)
    
    retreives_prefix = "metrics_3-"
    n_retrieves = sum(1 for file in files
                if re.match(rf"{retreives_prefix}\d+_node-1\.txt$", file) and os.path.isfile(os.path.join(data_dir, file)))
    print("n_retrieves: ", n_retrieves)

    results = {}
        
    for time_index in range(n_times):
        for retrieve_index in range(n_retrieves):
            print(f'{time_index}-{retrieve_index}_config.yaml')
            with open(os.path.join(data_dir, f'{time_index}-{retrieve_index}_config.yaml'), 'r') as file:
                data = yaml.safe_load(file)

            time_pull = data["time_pull"]
            max_retrieve = data["max_retrieve"]

            print(f"Getting metrics data from run time pull {time_pull} (index {time_index}) max retrieve {max_retrieve} (index {retrieve_index})")

            if time_pull not in results:
                results[time_pull] = {max_retrieve: {}}
            else:
                results[time_pull][max_retrieve] = {}
            

            indices_to_remove = []
            if not get_metrics_data(data_dir, path_length, results[time_pull][max_retrieve], time_index, retrieve_index):
                indices_to_remove.append(max_retrieve)
                
            print(indices_to_remove)
            for index in indices_to_remove:
                results[time_pull][max_retrieve].pop(index, None)

        print(results[time_pull][max_retrieve].keys())

    with open(os.path.join(base_path, 'raw_metrics.json'), 'w') as f:
        json.dump(results, f, indent=2)

if __name__ == "__main__":
    main()