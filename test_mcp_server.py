#!/usr/bin/env python3
"""
Comprehensive test suite for context-mcp MCP server
Tests all tools, benchmarks performance, and generates assessment report
"""

import asyncio
import json
import subprocess
import time
import statistics
from datetime import datetime
from typing import Dict, List, Any, Optional
import sys

class MCPClient:
    """Simple MCP client using stdio transport"""
    
    def __init__(self, command: List[str]):
        self.process = None
        self.command = command
        self.request_id = 0
        
    async def start(self):
        """Start the MCP server process"""
        self.process = await asyncio.create_subprocess_exec(
            *self.command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE  # Capture stderr separately
        )
        print(f"✓ Started MCP server: {' '.join(self.command)}")
        
        # Initialize connection
        await self.call_method("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0"}
        })
        
    async def stop(self):
        """Stop the MCP server process"""
        if self.process:
            self.process.terminate()
            await self.process.wait()
            print("✓ Stopped MCP server")
    
    async def call_method(self, method: str, params: Dict = None) -> Dict:
        """Call an MCP method"""
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params or {}
        }
        
        # Send request
        request_str = json.dumps(request) + "\n"
        self.process.stdin.write(request_str.encode())
        await self.process.stdin.drain()
        
        # Read response (skip any non-JSON lines)
        max_attempts = 10
        for attempt in range(max_attempts):
            response_line = await self.process.stdout.readline()
            response_str = response_line.decode().strip()
            
            if not response_str:
                continue
                
            try:
                response = json.loads(response_str)
                break
            except json.JSONDecodeError:
                # Skip non-JSON lines (like log messages)
                if attempt == max_attempts - 1:
                    raise Exception(f"Could not parse response after {max_attempts} attempts")
                continue
        
        if "error" in response:
            raise Exception(f"MCP error: {response['error']}")
            
        return response.get("result", {})
    
    async def call_tool(self, tool_name: str, arguments: Dict = None) -> Dict:
        """Call an MCP tool"""
        result = await self.call_method("tools/call", {
            "name": tool_name,
            "arguments": arguments or {}
        })
        return result

class TestResults:
    """Store and format test results"""
    
    def __init__(self):
        self.tests_passed = 0
        self.tests_failed = 0
        self.errors = []
        self.benchmarks = {}
        self.metrics = {}
        self.stored_ids = []
        
    def add_success(self, test_name: str):
        self.tests_passed += 1
        print(f"  ✓ {test_name}")
        
    def add_failure(self, test_name: str, error: str):
        self.tests_failed += 1
        self.errors.append(f"{test_name}: {error}")
        print(f"  ✗ {test_name}: {error}")
        
    def add_benchmark(self, operation: str, timings: List[float], unit: str = "ms"):
        self.benchmarks[operation] = {
            "min": min(timings),
            "max": max(timings),
            "mean": statistics.mean(timings),
            "median": statistics.median(timings),
            "stdev": statistics.stdev(timings) if len(timings) > 1 else 0,
            "samples": len(timings),
            "unit": unit
        }
        
    def add_metric(self, name: str, value: Any, unit: str = ""):
        self.metrics[name] = {"value": value, "unit": unit}
        
    def print_report(self):
        """Print comprehensive assessment report"""
        print("\n" + "="*80)
        print(" CONTEXT-MCP SERVER ASSESSMENT REPORT")
        print("="*80)
        print(f"\nTest Execution: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        
        # Test Summary
        print(f"\n{'TEST SUMMARY':-^80}")
        total = self.tests_passed + self.tests_failed
        if total > 0:
            print(f"  Total Tests: {total}")
            print(f"  Passed: {self.tests_passed} ({self.tests_passed/total*100:.1f}%)")
            print(f"  Failed: {self.tests_failed} ({self.tests_failed/total*100:.1f}%)")
        else:
            print(f"  No tests were executed")
            return
        
        if self.errors:
            print(f"\n{'ERRORS':-^80}")
            for error in self.errors:
                print(f"  • {error}")
        
        # Performance Benchmarks
        print(f"\n{'PERFORMANCE BENCHMARKS':-^80}")
        print(f"  {'Operation':<25} {'Mean':<12} {'Median':<12} {'Min':<12} {'Max':<12} {'StdDev':<10}")
        print(f"  {'-'*25} {'-'*12} {'-'*12} {'-'*12} {'-'*12} {'-'*10}")
        
        for op, stats in self.benchmarks.items():
            unit = stats['unit']
            print(f"  {op:<25} "
                  f"{stats['mean']:.2f}{unit:<8} "
                  f"{stats['median']:.2f}{unit:<8} "
                  f"{stats['min']:.2f}{unit:<8} "
                  f"{stats['max']:.2f}{unit:<8} "
                  f"{stats['stdev']:.2f}{unit:<6}")
        
        # Key Metrics
        print(f"\n{'KEY METRICS':-^80}")
        for name, data in self.metrics.items():
            value = data['value']
            unit = data['unit']
            if isinstance(value, float):
                print(f"  {name}: {value:.2f} {unit}")
            else:
                print(f"  {name}: {value} {unit}")
        
        print("\n" + "="*80)


async def run_test_suite():
    """Execute comprehensive test suite"""
    results = TestResults()
    
    # Start MCP server
    client = MCPClient(["/home/kang/.local/bin/context-mcp", "--stdio"])
    
    try:
        await client.start()
        
        # Test 1: List available tools
        print("\n[1] Testing Tool Discovery")
        start = time.time()
        tools_result = await client.call_method("tools/list")
        tools = tools_result.get("tools", [])
        results.add_metric("Tool Discovery Time", (time.time() - start) * 1000, "ms")
        results.add_metric("Available Tools", len(tools))
        
        if len(tools) == 9:
            results.add_success("Tool discovery - expected 9 tools")
            print(f"    Available tools: {', '.join([t['name'] for t in tools])}")
        else:
            results.add_failure("Tool discovery", f"Expected 9 tools, got {len(tools)}")
        
        # Test 2: Store contexts with diverse content
        print("\n[2] Testing Storage Operations")
        test_contexts = [
            {
                "content": "Python async/await pattern for concurrent I/O operations",
                "domain": "Code",
                "tags": ["python", "async", "patterns"],
                "importance": 0.8,
                "source": "documentation"
            },
            {
                "content": "The Rust ownership model prevents data races at compile time through the borrow checker",
                "domain": "Documentation",
                "tags": ["rust", "memory-safety", "ownership"],
                "importance": 0.9,
                "source": "rust-book"
            },
            {
                "content": "Machine learning models require proper train/test split to avoid overfitting",
                "domain": "Research",
                "tags": ["ml", "validation", "best-practices"],
                "importance": 0.7,
                "source": "research-paper"
            },
            {
                "content": "REST API design: use proper HTTP methods, status codes, and resource naming conventions",
                "domain": "Code",
                "tags": ["api", "rest", "design"],
                "importance": 0.75,
                "source": "api-guidelines"
            },
            {
                "content": "Git rebase vs merge: rebase for clean history, merge for preserving context",
                "domain": "General",
                "tags": ["git", "version-control", "workflow"],
                "importance": 0.6,
                "source": "git-docs"
            },
        ]
        
        store_timings = []
        for i, ctx in enumerate(test_contexts):
            start = time.time()
            result = await client.call_tool("store_context", ctx)
            elapsed = (time.time() - start) * 1000
            store_timings.append(elapsed)
            
            content = result.get("content", [{}])[0]
            if content.get("type") == "text":
                text = content.get("text", "")
                # Extract ID from response - base64 encoded
                import re
                # Pattern for base64 ID: "id": "abc123==" or ID: abc123==
                id_match = re.search(r'"id":\s*"([A-Za-z0-9+/=]+)"', text)
                if not id_match:
                    id_match = re.search(r'ID:\s*([A-Za-z0-9+/=]+)', text, re.IGNORECASE)
                
                if id_match:
                    ctx_id = id_match.group(1)
                    results.stored_ids.append(ctx_id)
                    results.add_success(f"Store context #{i+1} (ID: {ctx_id[:8]}...)")
                else:
                    results.add_success(f"Store context #{i+1}")
                    print(f"    Warning: Could not extract ID from response: {text[:100]}")
            else:
                results.add_failure(f"Store context #{i+1}", "Invalid response format")
        
        results.add_benchmark("Store Context", store_timings)
        results.add_metric("Contexts Stored", len(test_contexts))
        
        # Test 3: Retrieve stored contexts
        print("\n[3] Testing Retrieval Operations")
        if results.stored_ids:
            retrieve_timings = []
            for i, ctx_id in enumerate(results.stored_ids[:3]):  # Test first 3
                start = time.time()
                result = await client.call_tool("get_context", {"id": ctx_id})
                elapsed = (time.time() - start) * 1000
                retrieve_timings.append(elapsed)
                
                content = result.get("content", [{}])[0]
                if content.get("type") == "text":
                    results.add_success(f"Retrieve context #{i+1}")
                else:
                    results.add_failure(f"Retrieve context #{i+1}", "Invalid response")
            
            results.add_benchmark("Retrieve Context", retrieve_timings)
        
        # Test 4: Query with filters
        print("\n[4] Testing Query Operations")
        query_tests = [
            {"domain": "Code", "test_name": "Query by domain (Code)"},
            {"tags": ["python", "rust"], "test_name": "Query by tags"},
            {"min_importance": 0.7, "test_name": "Query by importance threshold"},
            {"max_age_hours": 1, "test_name": "Query by temporal filter"},
        ]
        
        query_timings = []
        for query_test in query_tests:
            test_name = query_test.pop("test_name")
            start = time.time()
            result = await client.call_tool("query_contexts", query_test)
            elapsed = (time.time() - start) * 1000
            query_timings.append(elapsed)
            
            content = result.get("content", [{}])[0]
            if content.get("type") == "text":
                text = content.get("text", "")
                # Check if we got results
                if "found" in text.lower() or "contexts" in text.lower():
                    results.add_success(test_name)
                else:
                    results.add_failure(test_name, "No results indicator")
            else:
                results.add_failure(test_name, "Invalid response")
        
        results.add_benchmark("Query Contexts", query_timings)
        
        # Test 5: RAG Retrieval
        print("\n[5] Testing RAG Retrieval")
        rag_queries = [
            {"text": "python async patterns", "max_results": 5},
            {"text": "memory safety rust", "domain": "Documentation", "max_results": 3},
            {"text": "api design best practices", "min_importance": 0.7, "max_results": 5},
        ]
        
        rag_timings = []
        for i, query in enumerate(rag_queries):
            start = time.time()
            result = await client.call_tool("retrieve_contexts", query)
            elapsed = (time.time() - start) * 1000
            rag_timings.append(elapsed)
            
            content = result.get("content", [{}])[0]
            if content.get("type") == "text":
                results.add_success(f"RAG query #{i+1}")
            else:
                results.add_failure(f"RAG query #{i+1}", "Invalid response")
        
        results.add_benchmark("RAG Retrieval", rag_timings)
        
        # Test 6: Screening status update
        print("\n[6] Testing Screening Status")
        if results.stored_ids:
            start = time.time()
            result = await client.call_tool("update_screening", {
                "id": results.stored_ids[0],
                "status": "Safe",
                "reason": "Automated test verification"
            })
            elapsed = (time.time() - start) * 1000
            
            content = result.get("content", [{}])[0]
            if content.get("type") == "text":
                results.add_success("Update screening status")
                results.add_benchmark("Update Screening", [elapsed])
            else:
                results.add_failure("Update screening status", "Invalid response")
        
        # Test 7: Storage statistics
        print("\n[7] Testing Storage Statistics")
        start = time.time()
        result = await client.call_tool("get_storage_stats", {})
        elapsed = (time.time() - start) * 1000
        
        content = result.get("content", [{}])[0]
        if content.get("type") == "text":
            text = content.get("text", "")
            results.add_success("Get storage stats")
            results.add_benchmark("Get Storage Stats", [elapsed])
            print(f"    {text[:200]}...")
        else:
            results.add_failure("Get storage stats", "Invalid response")
        
        # Test 8: Temporal statistics
        print("\n[8] Testing Temporal Statistics")
        start = time.time()
        result = await client.call_tool("get_temporal_stats", {})
        elapsed = (time.time() - start) * 1000
        
        content = result.get("content", [{}])[0]
        if content.get("type") == "text":
            results.add_success("Get temporal stats")
            results.add_benchmark("Get Temporal Stats", [elapsed])
        else:
            results.add_failure("Get temporal stats", "Invalid response")
        
        # Test 9: Cleanup expired
        print("\n[9] Testing Cleanup Operations")
        start = time.time()
        result = await client.call_tool("cleanup_expired", {})
        elapsed = (time.time() - start) * 1000
        
        content = result.get("content", [{}])[0]
        if content.get("type") == "text":
            results.add_success("Cleanup expired contexts")
            results.add_benchmark("Cleanup Expired", [elapsed])
        else:
            results.add_failure("Cleanup expired", "Invalid response")
        
        # Test 10: Delete context
        print("\n[10] Testing Delete Operations")
        if results.stored_ids and len(results.stored_ids) > 0:
            delete_timings = []
            for i, ctx_id in enumerate(results.stored_ids[:2]):  # Delete first 2
                start = time.time()
                result = await client.call_tool("delete_context", {"id": ctx_id})
                elapsed = (time.time() - start) * 1000
                delete_timings.append(elapsed)
                
                content = result.get("content", [{}])[0]
                if content.get("type") == "text":
                    results.add_success(f"Delete context #{i+1}")
                else:
                    results.add_failure(f"Delete context #{i+1}", "Invalid response")
            
            results.add_benchmark("Delete Context", delete_timings)
        
        # Throughput test
        print("\n[11] Testing Throughput")
        num_ops = 50
        print(f"    Performing {num_ops} rapid store operations...")
        throughput_start = time.time()
        
        for i in range(num_ops):
            await client.call_tool("store_context", {
                "content": f"Throughput test content item {i}",
                "domain": "General",
                "importance": 0.5
            })
        
        throughput_elapsed = time.time() - throughput_start
        ops_per_sec = num_ops / throughput_elapsed
        results.add_metric("Store Throughput", ops_per_sec, "ops/sec")
        results.add_metric("Store Latency (avg)", (throughput_elapsed / num_ops) * 1000, "ms")
        results.add_success(f"Throughput test ({num_ops} operations)")
        
    except Exception as e:
        print(f"\n✗ Fatal error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await client.stop()
    
    # Print comprehensive report
    results.print_report()
    
    return results.tests_failed == 0

if __name__ == "__main__":
    success = asyncio.run(run_test_suite())
    sys.exit(0 if success else 1)
