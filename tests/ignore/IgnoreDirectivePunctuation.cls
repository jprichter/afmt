public class IgnoreDirectivePunctuation {
  void statements() {
    while (true) {
      // afmt:ignore
      break;
    }

    for (Integer i = 0; i < 3; i++) {
      // afmt:ignore
      continue;
    }

    // afmt:ignore
    return;
  }

  public String property {
    // afmt:ignore
    get;
    set;
  }
}
