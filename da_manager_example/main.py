import requests
import json
import time

L2_URL = 'http://localhost:3050'
DB_PATH = 'da_manager_example/data/pubdata_storage.json'
RETRIES = 3

def get_batch_pubdata(url, batch_number):
    headers = {"Content-Type": "application/json"}
    data = {"jsonrpc": "2.0", "id": 1, "method": "zks_getBatchPubdata", "params": [batch_number]}
    response = requests.post(url, headers=headers, data=json.dumps(data))
    print(f"response = {response}")
    return response.json()["result"]

def store_batch_pubdata(pubdata_storage, stored_pubdata, pubdata, batch_number):
    stored_pubdata[batch_number] = pubdata
    pubdata_storage.seek(0)
    json.dump(stored_pubdata, pubdata_storage)
    pubdata_storage.truncate()

def main():
    with open(DB_PATH, "r+") as pubdata_storage:
        try:
            stored_pubdata = json.load(pubdata_storage)
        except Exception:
            print(f"Error reading pubdata from pubdata_storage.json, starting from zero.")
            stored_pubdata = {}
        num_retries = 0
        starting_batch_id = len(stored_pubdata.keys()) + 1
        print(f"Starting from batch #{starting_batch_id}")
        while True:
            l1_batch_pubdata: list = get_batch_pubdata(L2_URL, starting_batch_id)
            if len(l1_batch_pubdata) == 0:
                if num_retries >= RETRIES: break
                print(f"Failed to get batch #{starting_batch_id} pubdata")
                print("Retrying in 60 seconds")
                time.sleep(60)
                num_retries += 1
                continue
            else:
                store_batch_pubdata(pubdata_storage, stored_pubdata, l1_batch_pubdata, starting_batch_id)
                print(f"Got batch #{starting_batch_id} pubdata")
                starting_batch_id += 1
                time.sleep(5)

if __name__ == '__main__':
    main()
