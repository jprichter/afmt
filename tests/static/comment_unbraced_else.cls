public class CommentUnbracedElse {
  public static void run(Integer symbol) {
    if (symbol == 16)
      symbol = 2;
    else if (symbol == 17) /* repeat zero 3..10 times */
      symbol = 3 + bits(symbol, 3);
    else /* == 18, repeat zero 11..138 times */
      symbol = 11 + bits(symbol, 7);
  }
}
