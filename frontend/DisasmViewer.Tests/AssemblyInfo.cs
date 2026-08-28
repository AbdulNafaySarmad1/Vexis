using Xunit;

// BackendLocatorTests.Find_PathFallback_SkipsRelativeEntriesButUsesAbsoluteOnes
// temporarily mutates the process-wide PATH environment variable. xunit
// parallelizes different test classes by default, and a concurrently-running
// test elsewhere that also relies on PATH lookup (e.g. anything touching
// BackendLocator.Find with no explicit override) could observe that mutated
// PATH mid-test and get a spurious result. The suite is small enough (well
// under a second total) that disabling collection-level parallelism is a
// negligible cost for eliminating that flake source entirely.
[assembly: CollectionBehavior(DisableTestParallelization = true)]
