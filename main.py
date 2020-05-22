with open("availability.txt") as my_file:
    stamps = my_file.readlines()
    stamps = list(map(lambda x: int(x.strip("\n")), stamps))

    length = len(stamps)
    first = stamps[0]
    up = []
    down = []
    for i in range(0, length - 1):
        if (stamps[i + 1] - stamps[i]) > 1:
            up.append((first, stamps[i]))
            down.append((stamps[i], stamps[i + 1]))
            first = stamps[i + 1]

    if (stamps[length - 1] - stamps[length - 2]) <= 1:
        up.append((first, stamps[length - 1]))

    print("up", up)
    print("down", down)
