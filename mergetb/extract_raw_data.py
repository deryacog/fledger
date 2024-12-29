import os
import re
import json
import sys
import yaml

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

def simulation_ran_successfully(data_dir, variable, index):
    dir = f"{data_dir}/{variable}"
    metrics_file = os.path.join(dir, f"metrics_{index}_node-1.txt")

    if not os.path.exists(metrics_file):
        return False

    with open(metrics_file, 'r') as f:
        content = f.read()

    if "loopix_number_of_proxy_requests" in content:
        return True
    else:
        return False

def create_results_dict(results, metrics_to_extract):
    for metric in metrics_to_extract:
        if metric == "loopix_bandwidth_bytes" or metric == "loopix_number_of_proxy_requests" or metric == "loopix_start_time_seconds" or metric == "loopix_incoming_messages":
            results[metric] = []
        else:
            results[metric] = {"sum": [], "count": []}

def get_bandwidth_bytes(results, content, metric, i):
    pattern = rf"{metric}\s+([0-9.e+-]+)$"
    match = re.search(pattern, content, re.MULTILINE)
    if match:
        results[metric].append(float(match.group(1)))
    else:
        print(f"Error for node-{i}: match {match}")

def get_proxy_request_or_start_time(results, content, metric, i):
    pattern = rf"{metric}\s+([0-9.e+-]+)$"
    match = re.search(pattern, content, re.MULTILINE)
    if match:
        results[metric].append(float(match.group(1)))
    else:
        print(f"Error for node-{i}: match {match}")

def get_incoming_messages(results, content, metric, i):
    provider_pattern = "loopix_provider_delay_milliseconds"
    client_pattern = "loopix_number_of_proxy_requests"
    if not provider_pattern in content and not client_pattern in content:
        pattern = rf"{metric}\s+([0-9.e+-]+)$"
        match = re.search(pattern, content, re.MULTILINE)

        results[metric].append(float(match.group(1)))

def get_histogram_metrics(results, content, metric, i):
    pattern_sum = rf"^{metric}_sum\s+([0-9.e+-]+)"
    pattern_count = rf"^{metric}_count\s+([0-9.e+-]+)"
    match_sum = re.search(pattern_sum, content, re.MULTILINE)
    match_count = re.search(pattern_count, content, re.MULTILINE)
    if match_count and match_sum:
        results[metric][f"sum"].append(float(match_sum.group(1)))
        results[metric][f"count"].append(float(match_count.group(1)))
    elif match_count or match_sum:
        print(f"Error for node-{i}: match_sum {match_sum}, match_count {match_count}")

def get_metrics_data(data_dir, path_length, results, variable, index):

    if not simulation_ran_successfully(data_dir, variable, index):
        print(f"Skipping run {variable} {index}, no end-to-end latency data found")
        return False

    create_results_dict(results, metrics_to_extract)

    for i in range(path_length*path_length + path_length * 2):
        print(f"Getting metrics data for node-{i}")
        dir = f"{data_dir}/{variable}"
        metrics_file = os.path.join(dir, f"metrics_{index}_node-{i}.txt")
        
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

    return True
          
def main():
    if len(sys.argv) != 3:
        print("Usage: python extract_raw_data.py <data_dir> <path_length>")
        sys.exit(1)

    base_path = sys.argv[1]
    data_dir = os.path.join(base_path, "raw_data")
    path_length = int(sys.argv[2])

    results = {}
    variables = [d for d in os.listdir(data_dir) if os.path.isdir(os.path.join(data_dir, d))]
    print(variables)

    for variable in variables:

        directory = f"{data_dir}/{variable}"
        suffix = "_node-0.txt"

        files = os.listdir(directory)

        runs = sum(1 for file in files
                    if file.endswith(suffix) and os.path.isfile(os.path.join(directory, file)))

        print(runs)

        if runs > 0:
            indices_to_remove = []
            for index in range(runs):
                with open(os.path.join(directory, f'{index}_config.yaml'), 'r') as f:
                    config = yaml.safe_load(f)

                print(variable == 'control')

                if variable == 'control':
                    run_value = index
                else:
                    run_value = config[variable]

                if variable not in results.keys():
                    results[variable] = {str(run_value): {}}
                else:
                    results[variable][str(run_value)] = {}

                print(f"Getting metrics data from run {variable} {index}")

                if not get_metrics_data(data_dir, path_length, results[variable][str(run_value)], variable, index):
                    indices_to_remove.append(index)
                    

        print(results)
    with open(f'{base_path}/raw_metrics.json', 'w') as f:
        json.dump(results, f, indent=2)

if __name__ == "__main__":
    main()