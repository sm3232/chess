a=$(wc -l 'bench.no_asm.txt'|awk '{print $1}')
echo $a
while [ $a -lt 1000 ]; do

# cargo build -r --features use_asm;
# for i in $(seq 1 2); do 
  # './target/release/main';
# done
# cargo build -r;
  './target/release/main';
# done

done
