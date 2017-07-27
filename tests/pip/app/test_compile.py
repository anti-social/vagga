
# Test Python3 PYTHONHASHSEED
GREETINGS = {'Hello', 'Bonjour', 'Привет', 'Вітаннячко'}


if __name__ == '__main__':
    print('{}, %username%!'.format(next(iter(GREETINGS))))
