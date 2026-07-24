public class CommentMethodChainMixedBlockLine {
  public static void run() {
    Builder b = new Builder()
      /* block before line */
      // line before method
      .build();
  }
}
