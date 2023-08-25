#! /bin/bash

cd "$(dirname "$0")"
shopt -s extglob

cd ..
cargo build --release
cd tests

result=0

while IFS= read -r script; do
    echo ">> Testing: $script"
    bash "$script" > /dev/null 
    
    if [ $? -ne 0 ]; then
        echo "> FAILED!"
        result=1
        break
    else
        echo "> Success"
        rm -rf environment/!(.gitignore)
    fi
done < <(find tests -type f -name "*.sh" | sort -V)

rm -rf environment/!(.gitignore)

exit $result
