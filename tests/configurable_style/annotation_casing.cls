@RestResource(urlMapping='/v1/*')
@IsTest
global class AnnotationCasing
{
	@AuraEnabled
	public static void run()
	{
	}

	@TestSetup
	static void setup()
	{
	}

	@Future
	static void later()
	{
	}

	@MyCustomAnno
	public static void custom()
	{
	}
}
