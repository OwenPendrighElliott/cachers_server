import time
import random
import pycurl
import io
import json
import concurrent.futures

BASE_URL = "http://127.0.0.1:8080"

def do_request(method, url, data=None, json_data=None):
    buffer = io.BytesIO()
    c = pycurl.Curl()
    c.setopt(c.URL, url)
    c.setopt(c.WRITEDATA, buffer)
    headers = []
    if json_data is not None:
        data = json.dumps(json_data)
        headers.append("Content-Type: application/json")
    if headers:
        c.setopt(c.HTTPHEADER, headers)
    if method.upper() == "POST":
        c.setopt(c.POST, 1)
        if data is not None:
            c.setopt(c.POSTFIELDS, data)
    elif method.upper() == "PUT":
        c.setopt(c.CUSTOMREQUEST, "PUT")
        if data is not None:
            c.setopt(c.POSTFIELDS, data)
    elif method.upper() == "DELETE":
        c.setopt(c.CUSTOMREQUEST, "DELETE")
        if data is not None:
            c.setopt(c.POSTFIELDS, data)
    # GET requires no extra options.
    c.perform()
    status_code = c.getinfo(c.RESPONSE_CODE)
    c.close()
    return status_code, buffer.getvalue().decode('utf-8')

def create_cache():
    payload = {
        "name": "test_cache",
        "cache_type": "lru",
        "capacity": 1000 
    }
    status, body = do_request("POST", f"{BASE_URL}/cache/create", json_data=payload)
    print("Create Cache:", status, body)

def put_value(key, value):
    return do_request("PUT", f"{BASE_URL}/cache/test_cache/{key}", data=value)

def get_value(key):
    return do_request("GET", f"{BASE_URL}/cache/test_cache/{key}")

def delete_value(key):
    return do_request("DELETE", f"{BASE_URL}/cache/test_cache/{key}")

def simulate_operation(key_pool):
    op = random.choices(['GET', 'PUT', 'DELETE'], weights=[60, 30, 10], k=1)[0]
    key = random.choice(key_pool)
    start_time = time.perf_counter()
    if op == 'GET':
        end_time = time.perf_counter()
        return get_value(key), end_time - start_time
    elif op == 'PUT':
        value = f"value_{random.randint(0, 10000)}"
        end_time = time.perf_counter()
        return put_value(key, value), end_time - start_time
    elif op == 'DELETE':
        end_time = time.perf_counter()
        return delete_value(key), end_time - start_time

def get_stats():
    status, body = do_request("GET", f"{BASE_URL}/cache/test_cache/stats")
    if status == 200:
        try:
            return json.loads(body)
        except Exception:
            return None
    return None

def prepopulate_keys(key_pool):
    for key in key_pool:
        put_value(key, "initial_value")

def main():
    create_cache()
    key_pool = [f"key{i}" for i in range(100)]
    prepopulate_keys(key_pool)
    
    num_operations = 20000  # Increase if you need more stress
    max_workers = 200

    print("Starting stress workload simulation with pycurl...")
    start_time = time.perf_counter()
    times = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(simulate_operation, key_pool) for _ in range(num_operations)]
        for future in concurrent.futures.as_completed(futures):
            try:
                _, time_taken = future.result()
                # Uncomment the next line to log each response:
                # print(res)
                times.append(time_taken)
            except Exception as e:
                print("Operation error:", e)
    total_time = time.perf_counter() - start_time
    print(f"Completed {num_operations} operations in {total_time:.2f} seconds")
    print(f"Throughput: {num_operations/total_time:.2f} ops/sec")
    
    print("Average operation time:", sum(times) / len(times))
    print("Max operation time:", max(times))
    print("Min operation time:", min(times))

    stats = get_stats()
    if stats:
        print("Cache Stats:", stats)
    else:
        print("Failed to retrieve cache stats.")

if __name__ == "__main__":
    main()
