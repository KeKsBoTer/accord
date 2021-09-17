


x = 10000
"ssh -f compute-2-1 "python3 $PWD/storagenode.py -p 10000 compute-0-2:10000 compute-0-3:10000"
ssh compute-2-1
cd starter_code/
python3 storagenode.py -p 10000"
