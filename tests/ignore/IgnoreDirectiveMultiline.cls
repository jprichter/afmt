public class IgnoreDirectiveMultiline {
  void run() {
    foo(
      bar,
      new Baz(
        // afmt:ignore
        a
  + b,
        qqqq
      ),
      zzzz
    );
  }
}
