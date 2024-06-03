# Take a list of dictionaries and return a dictionary of lists
def dl_to_ld(dl):
    ld = {}
    for d in dl:
        for k, v in d.items():
            if k not in ld:
                ld[k] = []
            ld[k].append(v)
    return ld
