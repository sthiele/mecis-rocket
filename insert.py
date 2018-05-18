
with open("Mecis_10000.csv") as f:
        line = f.readline()
        s = 0
        oldspecies = ""
        oldp1 = ""
        oldp2 = ""
        oldp3 = ""
        oldp4 = ""
        oldp5 = ""
        oldp6 = ""
        while line :
                line = line.strip()
                line = line.replace(" ", "")
                pars = line.split(',')
                species = pars[0]
                p1 = pars[1]
                p2 = pars[2]
                p3 = pars[3]
                p4 = pars[4]
                p5 = pars[5]
                p6 = pars[6]
                mcs= ""
                c = 1
                mcs = []
                for e in range(7,len(pars)) :
                        if pars[e] == '1' :
                          mcs.append(c)
                        c = c+1
                if oldspecies != species or oldp1 != p1 or oldp2 != p2 or oldp3 != p3 or oldp4 != p4 or oldp5 != p5 or oldp6 != p6 :
                        oldspecies = species
                        oldp1 = p1
                        oldp2 = p2
                        oldp3 = p3
                        oldp4 = p4
                        oldp5 = p5
                        oldp6 = p6
                        s = 1
                else:  s = s+1
                if len(mcs) == 0 :
                        line = f.readline()
                        continue
    # Compose SQL query
                sql = "INSERT into mis (organism, model, inreac, exreac, mby, mpy, scen, s, r) values ('"+str(species)+"', '"+str(p1)+"', '"+str(p2)+"', '"+str(p3)+"', '"+str(p4)+"', '"+str(p5)+"', '"+str(p6)+"', '"
                for i in range(0,len(mcs)):
                        val = sql + str(s) + "', '"+str(mcs[i])+ "');"
                        print(val)
    #SQL query to INSERT a record into the table FACTRESTTBL.
                        #cursor.execute(val)
    # Commit your changes in the database
                        #db.commit()
                line = f.readline()

# disconnect from server
#db.close()

