
import requests

print("insert node id")
while True:
    node = input() 
    try: 
        x = requests.get(f'http://localhost:{node}/neighbors')
        print(x)    
        print(f'http://localhost:{node}/neighbors')
    except Exception as e:
        print(f"{str(e)} on {node}")

        
    
