for i in {1..20}
do
sleep 20
./target/debug/pyrsia build docker --image alpine:3.16
done
