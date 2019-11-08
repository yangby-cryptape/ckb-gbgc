from hashlib import sha256

# the seed here is only an example.
# The seed for the lottery (of Nervos CKB Minning Competition between 6/15 and 6/28) will be the block hash of the block of height N.
# Here is the sha256 hash of the sentence that contains the information of N:0x24f7501665d4f59b7f65c0853f8dd2a68fe528d345ffe63721f391eec711c190
# Update: The message is "N = 77". According to the competition result, the block hash of height 77 is 0x73ba270324ee87ed8990acbc316380c584dea21a1b8b87f4e8c363595e08225f
seed = "0x73ba270324ee87ed8990acbc316380c584dea21a1b8b87f4e8c363595e08225f"

# This is the number of the participants in total.
# The number here should be replaced with the actual number after the competition conclude.
participantNumber = 21737

# init lottery and result
result = [1,2,3] # put 1,2,3 in the result array to avoid drawing them
hashed = sha256(seed).hexdigest()
lottery = int(int(hashed,16) % participantNumber)
result.append(lottery)

# loop to draw lottery
for i in range(63):
    while lottery in result:
        hashed = sha256(hashed).hexdigest()
        lottery = int(int(hashed,16) % participantNumber)
    result.append(lottery)

# sort the result
result.sort()

# remove 1,2,3 from the result array
result.remove(1)
result.remove(2)
result.remove(3)

# The result printed here will be a list of numbers, which indicates the lottery winners according to their rank upon the competition ending.
# The list that contains all the participants' address will be disclosed after the competition is finished.
print(result)


# The result is:
# [104, 557, 1261, 1363, 1906, 1981, 2067, 2463, 2841, 3233, 3397, 3424, 4163, 4197, 4546, 4571, 4818, 5027, 5051, 5085, 5093, 5121, 5618, 5709, 5969, 6057, 6279, 6348, 6556, 6562, 6804, 7216, 7845, 8272, 8303, 8982, 9583, 9833, 10516, 10750, 11105, 11539, 11748, 12284, 12591, 14682, 14885, 15416, 15532, 15577, 15879, 15921, 16457, 16502, 16785, 16934, 17042, 17811, 17974, 18959, 20173, 20312, 20912, 20933]
