#!/usr/bin/env node

// Extract calculateOptimalThreshold function for testing
function calculateOptimalThreshold(results) {
    if (!results || results.length === 0) return 0.3; // Minimum 30%

    // Sort results by similarity (highest first)
    const sortedResults = [...results].sort((a, b) => b.similarity - a.similarity);

    // Find threshold that gives us 5-10 results
    let targetCount = Math.min(8, Math.max(5, Math.floor(sortedResults.length * 0.1)));

    if (sortedResults.length <= targetCount) {
        // If we have fewer results than target, use a low threshold
        return Math.max(0.3, sortedResults[sortedResults.length - 1].similarity);
    }

    // Get the threshold at the target position
    let threshold = sortedResults[targetCount - 1].similarity;

    // Ensure we don't go below 30%
    threshold = Math.max(0.3, threshold);

    // Round down to 2 decimal places for more inclusive results
    threshold = Math.floor(threshold * 100) / 100;

    return threshold;
}

// Test suite
function runTests() {
    const tests = [
        {
            name: 'Empty results returns 0.3',
            input: [],
            expected: 0.3
        },
        {
            name: 'Few results returns minimum of last result or 0.3',
            input: [0.8, 0.7, 0.6, 0.5],
            expectedMin: 0.3,
            expectedMax: 0.5
        },
        {
            name: 'Many results calculates decile threshold',
            input: [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15, 0.1],
            expectedMin: 0.3,
            expectedMax: 0.7
        },
        {
            name: 'All low scores enforces 0.3 minimum',
            input: [0.25, 0.2, 0.15, 0.1, 0.05],
            expected: 0.3
        },
        {
            name: 'Very similar high scores',
            input: [0.85, 0.84, 0.83, 0.82, 0.81, 0.80, 0.79, 0.78],
            expectedMin: 0.78,
            expectedMax: 0.82
        },
        {
            name: 'Single result uses 0.3 minimum',
            input: [0.25],
            expected: 0.3
        },
        {
            name: 'Edge case - exactly 5 results',
            input: [0.9, 0.8, 0.7, 0.6, 0.5],
            expected: 0.5
        },
        {
            name: 'Edge case - exactly 8 results',
            input: [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.35, 0.3],
            expected: 0.5
        }
    ];

    console.log('🧪 Running calculateOptimalThreshold tests...\n');

    let passed = 0;
    let failed = 0;

    tests.forEach((test, index) => {
        const mockResults = test.input.map((sim, i) => ({ similarity: sim }));
        const result = calculateOptimalThreshold(mockResults);

        let testPassed = false;
        let expectedStr = '';

        if (test.expected !== undefined) {
            testPassed = result === test.expected;
            expectedStr = `${test.expected}`;
        } else {
            testPassed = result >= test.expectedMin && result <= test.expectedMax;
            expectedStr = `${test.expectedMin}-${test.expectedMax}`;
        }

        const status = testPassed ? '✅' : '❌';
        const passStr = testPassed ? 'PASS' : 'FAIL';

        console.log(`${index + 1}. ${test.name}`);
        console.log(`   Input: [${test.input.join(', ')}]`);
        console.log(`   Result: ${result}`);
        console.log(`   Expected: ${expectedStr}`);
        console.log(`   ${status} ${passStr}\n`);

        if (testPassed) {
            passed++;
        } else {
            failed++;
        }
    });

    console.log(`📊 Test Summary:`);
    console.log(`   ✅ Passed: ${passed}`);
    console.log(`   ❌ Failed: ${failed}`);
    console.log(`   📈 Total: ${tests.length}`);
    console.log(`   🎯 Success Rate: ${((passed / tests.length) * 100).toFixed(1)}%`);

    if (failed === 0) {
        console.log('\n🎉 All tests passed!');
        process.exit(0);
    } else {
        console.log('\n⚠️  Some tests failed');
        process.exit(1);
    }
}

// Run tests if this file is executed directly
if (require.main === module) {
    runTests();
}

module.exports = { calculateOptimalThreshold, runTests };