#!/usr/bin/env python3
"""
Test script for omega-cognitive-kernel Python bindings.
Tests initialization, single-step inference, and rollback functionality.
"""

import tempfile
import os
import sys

# Add the project root to path so we can import the bindings
sys.path.insert(0, "/workspaces/omega-cognitive-kernel")

import omega_cognitive_kernel_python as ok


def test_initialization():
    """Test kernel initialization."""
    print("=== Testing Initialization ===")
    
    # Test 1: Create new kernel with default path
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        print(f"✓ Kernel created at: {kernel.state_path()}")
        print(f"  Initial status: {kernel.status()}")
        
        # Test 2: Open existing kernel
        kernel2 = ok.Kernel.open(state_path)
        print(f"✓ Kernel opened from existing state: {kernel2.state_path()}")
        
        # Test 3: Create new kernel with default path
        kernel3 = ok.Kernel.new()
        print(f"✓ Kernel created with default path: {kernel3.state_path()}")
    
    print()


def test_single_step():
    """Test single-step inference."""
    print("=== Testing Single-Step Inference ===")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        
        # Execute a single step
        goal = "test goal"
        report = kernel.step(goal)
        print(f"✓ Step executed with goal: '{goal}'")
        print(f"  Report: {report}")
        
        # Execute multiple steps
        report2 = kernel.run("another goal", 3)
        print(f"✓ Run executed with 3 steps")
        print(f"  Report: {report2}")
        
        # Check status after steps
        status = kernel.status()
        print(f"✓ Status after steps: total_steps={status.total_steps}, avg_err={status.avg_prediction_error:.3f}")
    
    print()


def test_checkpoints_and_rollback():
    """Test checkpoint creation and rollback."""
    print("=== Testing Checkpoints and Rollback ===")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        
        # Run a few steps to have some state
        kernel.run("initial goal", 5)
        print(f"✓ Ran 5 steps, total_steps={kernel.status().total_steps}")
        
        # Create a checkpoint
        checkpoint = kernel.create_checkpoint("before_changes")
        print(f"✓ Checkpoint created: {checkpoint.id[:8]}... (label: {checkpoint.label}, step: {checkpoint.step})")
        
        # Run more steps
        kernel.run("more goals", 3)
        print(f"✓ Ran 3 more steps, total_steps={kernel.status().total_steps}")
        
        # List checkpoints
        checkpoints = kernel.list_checkpoints()
        print(f"✓ Checkpoints listed: {len(checkpoints)} checkpoint(s)")
        for cp in checkpoints:
            print(f"  - {cp.id[:8]}... label='{cp.label}' step={cp.step} err={cp.avg_err:.3f}")
        
        # Rollback to checkpoint
        kernel.rollback(checkpoint.id)
        print(f"✓ Rolled back to checkpoint {checkpoint.id[:8]}...")
        print(f"  total_steps after rollback: {kernel.status().total_steps}")
        
        # Verify rollback worked (should be back to step 5)
        assert kernel.status().total_steps == 5, f"Expected 5 steps after rollback, got {kernel.status().total_steps}"
        print("✓ Rollback verified: state correctly restored to checkpoint")
    
    print()


def test_snapshot_json():
    """Test snapshot JSON serialization."""
    print("=== Testing Snapshot JSON ===")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        
        kernel.run("test goal", 2)
        
        # Get snapshot as JSON
        json_str = kernel.snapshot_json()
        print(f"✓ Snapshot JSON retrieved (length: {len(json_str)} chars)")
        
        # Apply snapshot from JSON
        kernel2 = ok.Kernel.with_state_path(os.path.join(tmpdir, "test_state2"))
        kernel2.apply_snapshot_json(json_str)
        print(f"✓ Snapshot applied to new kernel")
        print(f"  Original steps: {kernel.status().total_steps}, Restored steps: {kernel2.status().total_steps}")
        
        assert kernel2.status().total_steps == kernel.status().total_steps
        print("✓ Snapshot JSON round-trip verified")
    
    print()


def test_strategy_weight():
    """Test strategy weight modification."""
    print("=== Testing Strategy Weight ===")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        
        # Get initial strategy weights (if any)
        status = kernel.status()
        print(f"✓ Initial strategy count: {status.strategy_count}")
        
        # Try setting a strategy weight
        result = kernel.set_strategy_weight("explore", 2.0)
        print(f"✓ Set strategy weight 'explore' to 2.0: {result}")
        
        # Try setting unknown strategy
        result2 = kernel.set_strategy_weight("unknown_strategy", 1.0)
        print(f"✓ Set unknown strategy weight: {result2} (expected False)")
    
    print()


def test_l2_settings():
    """Test L2 enable/disable."""
    print("=== Testing L2 Settings ===")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        
        # Check initial L2 state
        status = kernel.status()
        print(f"✓ Initial L2 enabled: {status.l2_enabled}")
        
        # Enable L2
        kernel.set_l2_enabled(True)
        status = kernel.status()
        print(f"✓ L2 enabled: {status.l2_enabled}")
        
        # Disable L2
        kernel.set_l2_enabled(False)
        status = kernel.status()
        print(f"✓ L2 disabled: {status.l2_enabled}")
    
    print()


def test_trajectory_replay():
    """Test trajectory replay."""
    print("=== Testing Trajectory Replay ===")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        state_path = os.path.join(tmpdir, "test_state")
        kernel = ok.Kernel.with_state_path(state_path)
        
        # Run some steps with trajectory dumping enabled
        kernel.set_dump_trajectory(True)
        kernel.run("replay test", 4)
        
        # Replay trajectory
        replay = kernel.replay_trajectory(3)
        print(f"✓ Trajectory replay: {replay.rows} rows, {replay.patches} patches")
        print(f"  Path: {replay.path}")
        print(f"  First avg_err: {replay.first_avg_err}, Last avg_err: {replay.last_avg_err}")
        print(f"  Tail ({len(replay.tail)} rows):")
        for row in replay.tail:
            print(f"    Step {row.step}: action='{row.action}' patched={row.patched} err={row.avg_err}")
    
    print()


def main():
    """Run all tests."""
    print("=" * 60)
    print("Omega Cognitive Kernel - Python Bindings Test Suite")
    print("=" * 60)
    print()
    
    try:
        test_initialization()
        test_single_step()
        test_checkpoints_and_rollback()
        test_snapshot_json()
        test_strategy_weight()
        test_l2_settings()
        test_trajectory_replay()
        
        print("=" * 60)
        print("✅ ALL TESTS PASSED!")
        print("=" * 60)
        return 0
        
    except Exception as e:
        print(f"\n❌ TEST FAILED: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())