public class CommentMethodChain {
  public static void run() {
    Builder b =
      new LogEntryEventBuilder(
        getUserSettings(),
        loggingLevel,
        shouldSaveThisEntryValue
      )
      // Don't include the logging system's classes in the stack trace
      .parseStackTrace(new System.DmlException().getStackTraceString());
  }
}
