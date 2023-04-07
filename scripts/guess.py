import random

while True:
    number = random.randint(1,10)
    attempts = 0

    while True:
        guess = int(input("Input a number between 1 and 10!"))
        attempts = attempts + 1

        if guess == number:
            print("Congrats!!! It took you " + str(attempts) + " attempts!")

            break
        else:
            print("Try again")
