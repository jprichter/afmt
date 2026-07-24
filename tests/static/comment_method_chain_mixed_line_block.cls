public class CommentMethodChainMixedLineBlock {
  public static void run() {
    Builder b = new Builder()
      // line before block
      /* block before method */
      .build();
  }
}
