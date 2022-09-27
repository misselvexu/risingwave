echo "--- set the FLINK_HOME value"
export FLINK_HOME=/home/ubuntu/flink

echo "--- starts both zookeeper and kafka"
./start_kafka.sh
sleep 5

if jps | grep 'QuorumPeerMain'; then
  printf "zookeeper started\n"
else
  printf "zookeeper did not start\n"
  exit 1
fi

if jps | grep 'Kafka'; then
  printf "kafka started\n"
else
  printf "kafka did not start\n"
  exit 1
fi

echo "--- start Node Exporter, Pushgateway, Prometheus and Grafana"
./start_four_monitoring_components.sh
sleep 5
if ./show_four_monitoring_components.sh | grep 'node_exporter'; then
  printf "node_exporter has started successfully\n"
else
  printf "node_exporter did not start\n"
  exit 1
fi

if ./show_four_monitoring_components.sh | grep 'grafana-server'; then
  printf "grafana-server has started successfully\n"
else
  printf "grafana-server did not start\n"
  exit 1
fi

if ./show_four_monitoring_components.sh | grep 'pushgateway'; then
  printf "pushgateway has started successfully\n"
else
  printf "pushgateway did not start\n"
  exit 1
fi

if ./show_four_monitoring_components.sh | grep 'prometheus'; then
  printf "prometheus has started successfully\n"
else
  printf "prometheus did not start\n"
  exit 1
fi

echo "--- starts both the Flink cluster and Nexmark monitoring service"
./restart-flink.sh
sleep 5
if jps | grep -e "CpuMetricSender" -e "TaskManagerRunner" > /dev/null; then
  printf "flink cluster and nexmark started."
  printf "TaskManagerRunner represents the Flink and CpuMetricSender represents Nexmark monitoring service\n"
else
  printf "flink cluster and nexmark did not start\n"
  exit 1
fi

echo "--- insert kafka from flink datagen source"
./run_insert_kafka.sh
printf "insert kafka has finished\n"

echo "--- check the number of records in the kafka topic"
./show_kafka_topic_records.sh "nexmark"

echo "--- run the benchmark $1 in kafka source"
./run_kafka_source.sh "${1:-q0,q1,q2,q3,q4,q5,q6,q7,q8,q9,q10,q11,q12,q13,q14,q15,q16,q17,q18,q19,q20,q21,q22}"
printf "completed the benchmark for all the queries\n"

echo "--- run the benchmark $1 in datagen source"
./run_datagen_source.sh "${1:-q0,q1,q2,q3,q4,q5,q6,q7,q8,q9,q10,q11,q12,q13,q14,q15,q16,q17,q18,q19,q20,q21,q22}"

echo "--- restart the flink for the graceful start of next benchmark"
./restart-flink.sh
