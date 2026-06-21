echo "=== 🐚 BYOShell Feature Showcase ==="

echo ""
echo "--- 1. Basic Built-in Commands ---"
echo "Current directory:"
pwd
echo "Testing 'type' command:"
type echo
type ls
type nonexistent

echo ""
echo "--- 2. Variable Declaration & Expansion ---"
declare DEMO_VAR="BYOShell"
echo "Hello from $DEMO_VAR!"
echo "Testing curly brace expansion: ${DEMO_VAR}_rocks"

echo ""
echo "--- 3. Pipelines ---"
echo "Testing a simple pipeline:"
echo "apple,banana,cherry" | cat
echo "Testing a multi-stage pipeline:"
ls | head -n 5 | cat

echo ""
echo "--- 4. I/O Redirection ---"
echo "Writing stdout to a file..." > showcase_out.txt
echo "Appending to the same file..." >> showcase_out.txt
echo "Contents of showcase_out.txt:" | cat
cat showcase_out.txt

echo "Redirecting stderr..."
ls nonexistent_directory 2> showcase_err.txt
echo "Contents of showcase_err.txt:" | cat
cat showcase_err.txt

echo ""
echo "--- 5. Background Jobs ---"
echo "Starting a sleep command in the background..."
sleep 3 &
jobs

echo ""
echo "--- 6. History ---"
history 3

echo ""
echo "=== Showcase Finished ==="
echo "Cleaning up temporary files..."
rm showcase_out.txt showcase_err.txt
echo "Done!"
