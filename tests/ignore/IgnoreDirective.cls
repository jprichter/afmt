public class IgnoreDirective {
  // keep this comment
  // afmt:ignore
  void   handFormatted( ) {

    // preserve this internal comment

    List<String> values = new List<String>();

    values.add('one')  .add('two');
  }

  // afmt:ignore
  void   builder( ) { return; }

  void formatted() {
    // afmt:ignore
    Integer   value=  2;

    Integer value = 1;
  }
}
