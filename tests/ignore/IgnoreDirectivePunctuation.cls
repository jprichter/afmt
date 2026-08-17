public class IgnoreDirectivePunctuation {
  void statements() {
    while (true) {
      break;
    }

    for (Integer i = 0; i < 3; i++) {
      continue;
    }

    return;
  }

  public String property { get; set; }
}
