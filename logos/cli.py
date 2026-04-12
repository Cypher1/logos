import sys
from pathlib import Path
from prompt_toolkit import PromptSession

from rich.console import Console

from logos.bot import Bot
import logos.tools as tools

MODEL='gemma4:latest'


def main():
    console = Console()
    session = PromptSession()

    assistant = Bot(MODEL) \
        .add_tool(tools.get_temperature) \
        .add_tool(tools.get_conditions)

    # Load in the previous session
    state_file = Path.home() / ".logos.jsonl"
    assistant.load_state(state_file)

    while True:
        try:
            console.clear()
            for message in assistant.messages:
                assistant.render_message(console, message)

            last = assistant.messages[-1] if assistant.messages else None
            if last is None or (last.role != 'user' and last.role != 'tool' and last.content):
                console.print('user', style='red')
                response = session.prompt('> ')
                if response:
                    assistant.add_message('user', response)
            else:
                assistant.get_response()
        except KeyboardInterrupt:
            break
        except EOFError:
            break
        except Exception as e:
            print(e, file=sys.stderr)
            # Then quit

    assistant.save_state(state_file)


if __name__ == '__main__':
    main()
