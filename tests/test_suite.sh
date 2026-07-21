echo "--- BYOShell Feature Showcase ---"
echo "[SETUP] workspace"
rm -f test_output.txt
rm -f test_errors.txt
rm -f history_snapshot.txt
mkdir -p byoshell_test/subdir/nested
pwd

echo "[TEST] ls, mkdir, touch"
touch byoshell_test/file1.txt
touch byoshell_test/subdir/nested/file2.txt
ls byoshell_test
ls -la byoshell_test
ls -l byoshell_test/subdir

echo "[TEST] cd and pwd"
cd byoshell_test
pwd
cd subdir
pwd
cd ..
cd ..
pwd

echo "[TEST] declare, expansion, quoting"
declare PROJECT_NAME=BYOShell
declare TARGET_DIR=byoshell_test
declare -p PROJECT_NAME
echo "Project: $PROJECT_NAME"
echo '${PROJECT_NAME} stays literal in single quotes'
echo "${PROJECT_NAME}_workspace"
echo path:$TARGET_DIR/subdir/nested

echo "[TEST] redirection and cat"
echo "line one" > byoshell_test/file1.txt
echo "line two" >> byoshell_test/file1.txt
cat byoshell_test/file1.txt
cat byoshell_test/missing.txt 2> test_errors.txt
cat test_errors.txt

echo "[TEST] type builtin and external lookup"
type echo
type sort
type definitely_missing_command

echo "[TEST] custom completion registration"
complete -C tests/completions/demo_completion.cmd democtl
complete -p democtl
complete
complete -r democtl
echo "completion removed for democtl"
complete

echo "[TEST] history"
history 8
history -w history_snapshot.txt
cat history_snapshot.txt

echo "[TEST] pipelines"
echo "Pipeline support is currently implemented on Unix builds."
echo "Example command: sort byoshell_test/pipeline.txt | sort"

echo "[TEST] background jobs"
cargo --version &
jobs
echo "done waiting for background status"
jobs

echo "[TEST] rm cleanup"
rm -f test_output.txt
rm -f test_errors.txt
rm -f history_snapshot.txt
rm -r byoshell_test
ls

echo "--- SHOWCASE COMPLETE ---"
