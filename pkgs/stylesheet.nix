{
  lib,
  # build dependencies
  runCommandLocal,
  sassc,
  # options
  sasscExecutable ? lib.getExe sassc,
  sasscArguments ? ["-t expanded"],
  styleSheet ? ./assets/default-styles.scss,
}:
runCommandLocal "compile-css" {} ''
  mkdir -p $out
  ${sasscExecutable} ${lib.concatStringsSep " " sasscArguments} ${styleSheet} > $out/sys-docs-style.css
''
