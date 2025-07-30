def incomplete_function():
    raise NotImplementedError()

def debug_function():
    print("debug: entering function")

def bad_exception_handling():
    try:
        do_something()
    except:
        pass
EOF < /dev/null