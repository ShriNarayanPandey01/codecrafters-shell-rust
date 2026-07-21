case "$3" in
  democtl)
    printf '%s\n' start stop status config --help --env
    ;;
  --env)
    printf '%s\n' dev staging prod
    ;;
  config)
    printf '%s\n' show reset path
    ;;
  *)
    printf '%s\n' start stop status config --help --env
    ;;
esac
