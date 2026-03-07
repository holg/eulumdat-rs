"""Launch the photometric knowledge quiz.

Usage:
    python -m eulumdat_quiz          # Interactive text quiz (pure Python)
    python -m eulumdat_quiz --tui    # Launch TUI version (requires cargo install eulumdat-tui-quiz)
"""
import shutil
import subprocess
import sys


def run_tui():
    """Try to launch the ratatui TUI quiz binary."""
    binary = shutil.which("eulumdat-quiz")
    if binary is None:
        print("TUI quiz binary not found.")
        print("Install it with: cargo install eulumdat-tui-quiz")
        print("Falling back to text mode...\n")
        return False
    try:
        subprocess.run([binary] + sys.argv[2:], check=False)
        return True
    except (OSError, subprocess.SubprocessError) as e:
        print(f"Failed to launch TUI: {e}")
        print("Falling back to text mode...\n")
        return False


def run_text_quiz():
    """Run a simple text-based quiz using the Python bindings."""
    from . import QuizBank, QuizConfig, QuizSession

    print("=" * 60)
    print("  Photometric Knowledge Quiz")
    print("=" * 60)
    print(f"\n  {QuizBank.total_count()} questions available\n")

    categories = QuizBank.categories()
    for cat, count in categories:
        print(f"    {cat.label()}: {count}")

    print()
    try:
        n = input("How many questions? [10]: ").strip()
        num = int(n) if n else 10
    except (ValueError, EOFError):
        num = 10

    config = QuizConfig(num_questions=num)
    session = QuizSession(config)
    idx, total = session.progress()

    while not session.is_finished():
        q = session.current_question()
        idx, total = session.progress()
        print(f"\n--- Question {idx + 1} of {total} ---")
        print(f"[{q.category.label()}] [{q.difficulty.label()}]\n")
        print(f"  {q.text}\n")

        for i, opt in enumerate(q.options):
            print(f"    {chr(65 + i)}) {opt}")

        while True:
            try:
                ans = input("\nYour answer (A-D or S to skip): ").strip().upper()
            except EOFError:
                print("\nQuiz aborted.")
                return

            if ans == "S":
                session.skip()
                print("  Skipped.")
                break
            if ans in "ABCD" and len(ans) == 1:
                choice = ord(ans) - ord("A")
                result = session.answer(choice)
                if result.is_correct:
                    print("  Correct!")
                else:
                    correct_letter = chr(65 + result.correct_index)
                    print(f"  Wrong! The correct answer was {correct_letter}.")
                print(f"  {result.explanation}")
                if result.reference:
                    print(f"  Reference: {result.reference}")
                break
            print("  Please enter A, B, C, D, or S.")

    score = session.score()
    print("\n" + "=" * 60)
    print(f"  Final Score: {score.correct}/{score.total} ({score.percentage():.0f}%)")
    print("=" * 60)

    if score.by_category:
        print("\n  By Category:")
        for cs in score.by_category:
            print(f"    {cs.category.label()}: {cs.correct}/{cs.total}")

    if score.by_difficulty:
        print("\n  By Difficulty:")
        for ds in score.by_difficulty:
            print(f"    {ds.difficulty.label()}: {ds.correct}/{ds.total}")

    print()


def main():
    if "--tui" in sys.argv:
        if run_tui():
            return
    run_text_quiz()


if __name__ == "__main__":
    main()
