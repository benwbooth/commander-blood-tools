[0000]   BLOCK (exit -> @00F6)
[0004]     ENDIF
[0005]     rec_0F4C |= 0x2
[000A]     rec_0F82 |= 0x2
[000F]     rec_0D60 |= 0x2
[0014]     rec_096A |= 0x2
[0019]     rec_0C4C |= 0x2
[001E]     rec_0B74 |= 0x2
[0023]     rec_0BE0 |= 0x2
[0028]     rec_0F10 |= 0x2
[002D]     rec_09C4 |= 0x2
[0032]     rec_0B38 |= 0x2
[0037]     rec_0A30 |= 0x2
[003C]     rec_0DE4 |= 0x2
[0041]     rec_0E5C |= 0x2
[0046]     rec_0E98 |= 0x2
[004B]     rec_0EB6 |= 0x2
[0050]     rec_0874 |= 0x1
[0055]     rec_043C |= 0x1
[005A]     rec_088A = 3944
[005F]     blood.talk = 65535
[0064]     rec_037A = 65535
[0069]     rec_040A = 65535
[006E]     rec_04E2 = 65535
[0073]     rec_0452 = 3884
[0078]     rec_03C2 = 65535
[007D]     rec_0212 = 65535
[0082]     Sinox = sur.value
[0089]     rec_0602 = 3092
[008E]     rec_03C2 = 65535
[0093]     rec_0182 = 65535
[0098]     rec_025A = 65535
[009D]     rec_0572 = 65535
[00A2]     rec_05BA = 3422
[00A7]     rec_064A = 3884
[00AC]     compris = 1
[00B3]     vbio = 3
[00BA]     SETCHAR slot 1 = "match"
[00C3]     SETCHAR slot 2 = "ppit"
[00CB]     SETCHAR slot 3 = "hatetv"
[00D5]     SETCHAR slot 4 = "venus"
[00DE]     SETCHAR slot 5 = "scrut"
[00E7]     SETCHAR slot 6 = "present"
[00F2]     POKE [0x0001] = 0
  END
[00F6]   BLOCK (exit -> @0120)
[00FA]     AWAIT gameflag_274F
[00FB]     GUARD active_actor == Beauregard.talk (related 40)
[0100]     ENDIF
[0101]     SAY "Let me sleep ... Commander..."  '[voice 1]
[0113]     SAY "..."  '[voice 0, skip 1]
[011D]     END PRESENTATION Beauregard.talk
  END
[0120]   BLOCK (exit -> @014A)
[0124]     AWAIT gameflag_274F
[0125]     GUARD active_actor == Anna_Haf.talk (related 40)
[012A]     ENDIF
[012B]     SAY "Let me sleep ... Commander..."  '[voice 1]
[013D]     SAY "..."  '[skip 1]
[0147]     END PRESENTATION Anna_Haf.talk
  END
[014A]   BLOCK (exit -> @0170)
[014E]     AWAIT gameflag_274F
[014F]     GUARD active_actor == receiver.talk (related 40)
[0154]     ENDIF
[0155]     SAY "Shhhhhh ... Shhhhh..."
[0163]     SAY "..."  '[skip 1]
[016D]     END PRESENTATION receiver.talk
  END
[0170]   GOTO @019A
[0174]   AWAIT gameflag_274F
[0175]   GUARD active_actor == Morning_Oil.talk (related 40)
[017A]   ENDIF
[017B]   SAY "Let me sleep ... Commander..."  '[voice 1]
[018D]   SAY "..."  '[skip 1]
[0197]   END PRESENTATION Morning_Oil.talk
[019A]   BLOCK (exit -> @01A9)
[019E]     GUARD rec_103A == 3674
[01A3]     ENDIF
[01A4]     rec_06DA = 3674
  END
[01A9]   BLOCK (exit -> @01B8)
[01AD]     GUARD rec_103A == 2606
[01B2]     ENDIF
[01B3]     rec_06DA = 2606
  END
[01B8]   BLOCK (exit -> @0309)
[01BC]     AWAIT gameflag_252A
[01BD]     GUARD rec_103A == 3674
[01C2]     GUARD active_actor == Scruter_Mac.talk (related 40)
[01C7]     ENDIF
[01C8]     SAY "Turn around ... Move on ... There are too many pilgrims ... The planet's inaccessible ..."  '[voice 0, skip 1]
[01F0]     LOADSTR "1kkult20.hnm"
[01FF]     SAY "The planet KULT is inaccessible . There are too many pilgrims..."  '[voice 1]
[021D]     IF-BLOCK (exit -> @02FC)
[0220]       GUARD rec_06F8 < 3
[0227]       ENDIF
[0228]       SAY "Commander... Commander ... It's the planet TUMUL with a new name .."  '[skip 1]
[0248]       LOADSTR "1kkult20.hnm"
[0257]       SAY "That's right , Commander... When we saved BETAKAM IV, king of the Patagos, we altered history..."  '[skip 1]
[027F]       LOADSTR "hboc.hnm"
[028A]       SAY "Pilgrims now flock to hear the VOICE that saved BETAKAM and his people from the catastrophe..."  '[skip 1]
[02B2]       LOADSTR "hboc.hnm"
[02BD]       SAY "How about that ... What a story ..."
[02D5]       SAY "Turn around ... Keep moving... There are too many pilgrims... The planet is inaccessible..."  '[voice 0, skip 1]
[02F9]       END PRESENTATION Scruter_Mac.talk
    END
[02FC]     SAY "..."  '[skip 1]
[0306]     END PRESENTATION Scruter_Mac.talk
  END
[0309]   BLOCK (exit -> @052B)
[030D]     AWAIT gameflag_274F
[030E]     GUARD active_actor == ondoyant.talk (related 40)
[0313]     ENDIF
[0314]     SAY "..."  '[voice 17]
[031E]     SAY "..."  '[voice 18]
[0328]     SAY "I am yours to command , Commander ..."  '[voice 0]
[0340]     SAY "Commander ... My knees are weak ..."
[0356]     SAY "You want a little kiss ?"  '[voice 1]
[036A]     SAY "Holy hen gizzards , Commander ... I'm starting to overheat ..."
[0388]     SAY "You can talk to me Commander ... Tell me about yourself ..."  '[voice 2]
[03A8]     SAY "Where did she learn to swim like that ? ... You think she'd agree to give me lessons , Commander ?"
[03DA]     IF-BLOCK (exit -> @051E)
[03DD]       GUARD rec_07D0 > 0
[03E4]       GUARD bok == 1
[03EB]       ENDIF
[03EC]       SAY "Oh ! Commander , Honk told me you needed a ring ..."  '[voice 5]
[040C]       SAY "Commander , I swear I didn't say a thing ..."
[0428]       SAY "Ha ! Ha ! Who's a little fibber ? ... You ogle me as soon as the Commander's back is turned ..."  '[voice 6]
[045C]       SAY "Commander ... She's making this up ..."
[0472]       SAY "Really , Mister Honk ??? I'm a little disappointed in you ..."  '[skip 1]
[0492]       LOADSTR "rgbtr9.hnm"
[049F]       SAY "No no , Mister Bronko ... Don't believe a word she says ..."
[04C1]       SAY "Ha ! Ha ! Ha ! Ha..."  '[voice 4]
[04D7]       SAY "Commander ... I have a ring ... I want you to have it ... It's yours ..."  '[voice 6, skip 1]
[0501]       rec_12F0 = 65535
[0506]       SAY "Commander ... She gave us the ring ..."
    END
[051E]     SAY "..."  '[voice 20, skip 1]
[0528]     END PRESENTATION ondoyant.talk
  END
[052B]   BLOCK (exit -> @0719)
[052F]     AWAIT gameflag_252A
[0530]     GUARD rec_103A == 4024
[0535]     GUARD active_actor == Bug_Deluxe.talk (related 40)
[053A]     ENDIF
[053B]     SAY "You here , Consumer Comrade ?"  '[voice 0]
[054F]     SAY "Commander !!! It's Bug Deluxe , the sales assistant from VENUSIA ... How about that ..."
[0577]     SAY "Are you here for the wedding ? ..."  '[voice 2]
[058F]     SAY "Wedding ? What wedding ???"
[05A1]     SAY "You mean to say you haven't heard ?"  '[voice 3]
[05B9]     SAY "MIGRATOR and TINA BURNER are getting married ..."  '[voice 4]
[05D1]     SAY "They're giving a big show here at the BIG BAND CLUB..."  '[voice 5]
[05EF]     SAY "At the what ??? Was that BIG BAND or BIG BANG ???"
[060F]     SAY "Right here at the BIG BAND CLUB ! WITH A "D" AT THE END ..."  '[voice 6]
[0635]     SAY "This isn't right , Commander ... This is the BIG BAND , not the BIG BANG ..."
[065F]     SAY "Cap'n Bob isn't going to like this ..."
[0677]     SAY "What's going on here ??? What the blazes are we doing in a NIGHT CLUB ???"  '[skip 1]
[069F]     LOADSTR "boba2.hnm"
[06AB]     SAY "There's been a mix up , Cap'n Bob sir ..."
[06C7]     SAY "ALL OF YOU , INTO THE CRYOBOX RIGHT AWAY ..."  '[skip 2]
[06E3]     LOADSTR "bobb.hnm"
[06EE]     exp = 1
[06F5]     SAY "NOW ..."  '[skip 4]
[0701]     LOADSTR "bobb.hnm"
[070C]     rec_00AA = 3884
[0711]     rec_07B2 = 4024
[0716]     END PRESENTATION Bug_Deluxe.talk
  END
[0719]   BLOCK (exit -> @0B87)
[071D]     AWAIT gameflag_274F
[071E]     GUARD active_actor == Bob_Morlock.talk (related 40)
[0723]     GUARD exp == 1
[072A]     ENDIF
[072B]     SAY "What's the story ??? What's this BIG BANG business ???..."  '[voice 1]
[0747]     SAY "Did we get to the BIG BANG , yes or no ??? I'm listening , Commander ..."  '[voice 2]
[0771]     SAY "Uh , there's been a mix up , sir ..."
[078D]     SAY "I think OLGA the onboard interpreter mixed up BIG BANG and BIG BAND ..."
[07B1]     SAY "WHAT DO YOU MEAN SHE MIXED UP BIG BANG AND BIG BAND ... TELL ME THIS IS JUST A BAD DREAM !!!"  '[voice 2]
[07E5]     SAY "Afraid not , sir ... She mixed up BANG and BAND..."
[0803]     SAY "It's the only explanation ..."
[0815]     SAY "BIO-TRANSLATOR OLGA REPORTING :"
[0825]     SAY "MISTER HONK IS A DIRTY LIAR !!! I didn't mix anything up ..."
[0847]     SAY "OLGA ... CHILL OUT ... We can clear the question up real easy ... We'll just run a test ..."  '[voice 6]
[0877]     SAY "OLGA , say "BIG BANG" out loud three times ..."  '[voice 3]
[0893]     SAY "OLGA REPORTING :"
[08A1]     SAY "BIG BANG ... BIG BANG ... BIG BANG ..."
[08BB]     SAY "Okay ... Now say "BIG BAND" with a "D" ..."  '[voice 2]
[08D7]     SAY "OLGA REPORTING :"
[08E5]     SAY "BIG BANG ... BIG BANG ... BIG BANG ..."
[08FF]     SAY "Satisfied ?"
[090B]     SAY "Olga ... I said BIG BAND , not BIG BANG ... You mixed them up ... GOSH DARN IT !!!"  '[voice 3]
[093B]     SAY "What did I tell you ? Don't say I didn't ... She fouled up ..."
[0961]     SAY "She can't tell "D" from "G" ..."
[0977]     SAY "Honk ! Misbegotten junkheap ... It's all your fault !!! Check her multiplexers ..."  '[voice 5]
[099B]     SAY "Right away , Cap'n Bob sir ... I'm on it ..."
[09B9]     SAY "Ah !!! Commander ... Remind me to tell you how much I hate machines ..."  '[voice 2]
[09DF]     SAY "Got it ! Cap'n Bob... I found the glitch ... It's a MOSQUITO stuck to the letter "D" in the multiplexer ..."
[0A13]     SAY "I hate mosquitos ..."
[0A23]     SAY "MESSAGE FROM OLGA :"
[0A33]     SAY "IF THIS PLACE WAS CLEANED UP MORE OFTEN THIS KIND OF THING WOULDN'T HAPPEN ..."
[0A59]     SAY "She's right ... This Ark is a pigsty !!! This mess makes me sick !!!"  '[voice 7]
[0A7F]     SAY "Dont' look at me ! Mister BEAUREGARD's on cleaning duty this week !!!"
[0AA1]     SAY "What !!! Don't drag me into this ! That does it ! I've had it with this stupid ship ..."  '[skip 1]
[0AD1]     LOADSTR "hboc.hnm"
[0ADC]     SAY "I want to go home ..."  '[skip 1]
[0AF0]     LOADSTR "hboc.hnm"
[0AFB]     SAY "That's enough , Mister Beauregard ... Return to your cryobox ..."  '[voice 3]
[0B19]     SAY "I told you so ... I told you ..."
[0B33]     SAY "Commander, drop Mister BEAUREGARD off on the planet Tumul ..."  '[voice 5]
[0B4F]     SAY "AND GET BACK TO WORK !!!"  '[voice 2]
[0B63]     SAY "Yes sir , Cap'n Bob sir ..."  '[skip 3]
[0B79]     bok = 1
[0B80]     POKE [0x071A] = 0
[0B84]     END PRESENTATION Bob_Morlock.talk
  END
[0B87]   BLOCK (exit -> @0CCF)
[0B8B]     AWAIT gameflag_274F
[0B8C]     GUARD active_actor == Bob_Morlock.talk (related 40)
[0B91]     ENDIF
[0B92]     IF-BLOCK (exit -> @0CA7)
[0B95]       GUARD (airport.talk & 0x2) == 0
[0B9B]       ENDIF
[0B9C]       SAY "Aaaah ..."  '[voice 2]
[0BA8]       SAY "When will we get to the Big Bang ? It's URGENT , Commander ..."  '[voice 3]
[0BCC]       SAY "WE MUST FIND the Big Bang ... THAT'S YOUR MISSION , REMEMBER ?"  '[voice 4]
[0BEE]       SAY "Yes sir . All efforts are directed toward achieving the mission objective ..."
[0C10]       SAY "SILENCE HONK !!!"
[0C1E]       SAY "I'm mad as mad can be , Commander !!! You're flitting around like some gosh darn butterfly ..."  '[voice 5]
[0C4A]       SAY "You're using my time and money to go after ondoyants !!!"  '[voice 4]
[0C68]       SAY "That's what I call IN-TO-LE-RA-BLE !!!"  '[voice 5]
[0C7C]       SAY "I hope I've made my feelings clear ..."  '[voice 6]
[0C94]       SAY "Now leave me ..."  '[voice 6, skip 1]
[0CA4]       END PRESENTATION Bob_Morlock.talk
    END
[0CA7]     IF-BLOCK (exit -> @0CCF)
[0CAA]       GUARD (airport.talk & 0x2) != 0
[0CAF]       ENDIF
[0CB0]       SAY "Lets go to the Bigbang..."  '[voice 2]
[0CC2]       SAY "..."  '[skip 1]
[0CCC]       END PRESENTATION Bob_Morlock.talk
    END
  END
[0CCF]   BLOCK (exit -> @0EBA)
[0CD3]     AWAIT gameflag_252A
[0CD4]     GUARD rec_103A == 4024
[0CD9]     GUARD active_actor == Tina_Burner.talk (related 40)
[0CDE]     ENDIF
[0CDF]     SAY "Ohhh ... It's you , Commander ..."  '[voice 0]
[0CF5]     SAY "Oh no ... Her again ..."
[0D09]     SAY "AH ! How delightful to see you again , sweeties ... I so much wanted to invite you along ..."  '[voice 4]
[0D39]     SAY "But I didn't know where to send the invitations ..."  '[voice 6]
[0D55]     SAY "I'm getting married , you see ... Hee ! Hee ! Hee ! To darling MIGRATOR ... Hee ! Hee ! Hee !"  '[voice 6]
[0D8B]     SAY "Holy homeostatics ... Poor guy ..."
[0D9F]     SAY "We're giving a concert here at the BIG BAND CLUB ... It's such a fashionable place , you know ... Why , the whole galaxy'll be here !"  '[voice 7]
[0DDF]     SAY "Our concert's going to be broadcast by all the TV stations in the universe !!"  '[voice 8]
[0E05]     SAY "I'm so famous , Commander ... Hee ! Hee ! Hee !"  '[voice 9]
[0E25]     SAY "I can't help wishing we'd gone somewhere else ... This place has an end of the world feel ..."
[0E53]     SAY "Darling Migra's at the bar , drinking a frozen Migrita ..."  '[voice 2]
[0E71]     SAY "MIGRA OH MIGRA SWEETIE ... GUESS WHO'S COME ..."  '[voice 2]
[0E8B]     SAY "Go join him for a stiff one , Commander ..."  '[voice 1, skip 4]
[0EA7]     POKE [0x0CD0] = 0
[0EAB]     rec_0452 = 4060
[0EB0]     rec_07B2 = 3884
[0EB5]     OP_C1 C1 46 13 DC 0F
  END
[0EBA]   BLOCK (exit -> @1018)
[0EBE]     AWAIT gameflag_252A
[0EBF]     GUARD rec_103A == 4024
[0EC4]     GUARD active_actor == Migrator.talk (related 40)
[0EC9]     ENDIF
[0ECA]     SAY "You here , Commander ! My good friend ... Did Burny tell you the news ?"  '[voice 1]
[0EF2]     SAY "We're getting married ... Ah !!! She's so sexy , and smart ..."  '[voice 2]
[0F14]     SAY "Commander ... Howzabout a strategic withdrawal here ? ... Commander ?"
[0F32]     SAY "And it's all thanks to you ... I'd like you to be my best man !"  '[voice 5]
[0F5A]     SAY "Oh NO ..."
[0F68]     SAY "You'll give her the wedding band ..."  '[voice 6]
[0F7E]     SAY "THE WEDDING BAND ... How about a ring for her nose ..."
[0F9E]     SAY "Hurry up , Commander ... We can't have the ceremony without a ring ..."  '[voice 5]
[0FC2]     SAY "Holy handouts ... Where do we find a wedding band at this time of day or night ?"
[0FEE]     SAY "See you soon , Commander ..."  '[voice 2, skip 5]
[1002]     POKE [0x0EBB] = 0
[1006]     rec_0452 = 3884
[100B]     rec_07B2 = 4084
[1010]     rec_025A = 4024
[1015]     END PRESENTATION Migrator.talk
  END
[1018]   BLOCK (exit -> @115C)
[101C]     AWAIT gameflag_252A
[101D]     GUARD rec_103A == 4024
[1022]     GUARD active_actor == Yoko.talk (related 40)
[1027]     ENDIF
[1028]     SAY "Commander ! Yoko happy to see you again ..."  '[voice 1]
[1042]     SAY "Commander ... It's young Yoko ..."
[1056]     SAY "Migrator and Tina wait for wedding ring ..."  '[voice 2]
[106E]     IF-BLOCK (exit -> @10E3)
[1071]       GUARD rec_12F0 == 40
[1076]       ENDIF
[1077]       SAY "You have ring ?"  '[voice 5]
[1087]       SAY "No problem , Mister Yoko ... We have the wedding band right here ..."
[10AB]       SAY "YES !!! Me happy ..."  '[voice 6]
[10BD]       SAY "You go bar , Tina be impatient ..."  '[voice 7, skip 3]
[10D5]       POKE [0x1019] = 0
[10D9]       rec_00F2 = 4060
[10DE]       OP_C1 C1 46 13 DC 0F
    END
[10E3]     SAY "You agreed to get a ring , Commander . You'll have to deliver ..."
[1107]     SAY "Whatever it takes ..."
[1117]     SAY "You hurry , Commander . Everybody wait ..."  '[voice 4]
[112F]     SAY "Let's get to it , Commander . I want to know how this thing turns out ..."  '[skip 1]
[1159]     END PRESENTATION Yoko.talk
  END
[115C]   BLOCK (exit -> @1209)
[1160]     AWAIT gameflag_252A
[1161]     GUARD rec_103A == 4024
[1166]     GUARD active_actor == Fifi.talk (related 40)
[116B]     ENDIF
[116C]     SAY "Ga..."  '[voice 1]
[1176]     SAY "Ga ... Fifi eat ..."  '[voice 2]
[1188]     SAY "Fifi ! What are you doing here ?"
[11A0]     SAY "Fifi come for party ... Fifi like eat also ..."  '[voice 3]
[11BC]     SAY "Beautiful Tina look for you , Commander . She be at other bar ..."  '[voice 4]
[11E0]     SAY "You hurry , Commander ... Hee ! Hee! Hee!"  '[voice 5, skip 3]
[11FA]     rec_00F2 = 3884
[11FF]     rec_0452 = 4060
[1204]     OP_C1 C1 46 13 F4 0F
  END
[1209]   BLOCK (exit -> @143D)
[120D]     AWAIT gameflag_252A
[120E]     GUARD rec_103A == 4024
[1213]     GUARD rec_1340 == 4084
[1218]     GUARD active_actor == Tina_Burner.talk (related 40)
[121D]     ENDIF
[121E]     SAY "Ah !! There you are at last , Commander ... Everyone's waiting for you ..."  '[voice 1]
[1244]     SAY "Holy handstands !!!"
[1252]     SAY "MIGRA's so worried ... You do have the wedding band , Commander ?"  '[voice 2]
[1274]     SAY "Yeah ... We have your ring , Miss Burner ..."
[1290]     SAY "WHAT !! That cybernetic sex fiend is here ? No !! I don't want him at my wedding ..."  '[voice 4]
[12BE]     SAY "He's definitely not invited !!! I can't get married with him ogling me !!!"  '[voice 5]
[12E2]     SAY "WHAT ??? The old bag insulted me ... Are you gonna let her get away with that , Commander ?"
[1312]     SAY "Get lost , you old sow ! ... ..."
[132C]     SAY "SOW !! BOO HOO HOO ... MIGRA ... MIGRATOR ... Come ! I need you ... BOO HOO HOO ..."  '[voice 8]
[135C]     SAY "She's so gross , Commander ..."
[1370]     SAY "What's all this racket ? Can't a guy get any sleep around here ?"  '[skip 1]
[1394]     LOADSTR "bobb.hnm"
[139F]     SAY "She started it , Cap'n Bob sir ... Ask the Commander ..."
[13BF]     SAY "BOO HOO HOO"  '[voice 7]
[13CD]     SAY "Just keep the noise down , okay ?"  '[skip 1]
[13E5]     LOADSTR "bobb.hnm"
[13F0]     SAY "YES sir ... Right down ..."
[1404]     SAY "Okay ... Let's go see MIGRATOR , Commander ..."
[141E]     SAY "MIGRATOR ... BOO ... MIGRA HOO HOO HOO ..."  '[voice 4, skip 1]
[1438]     OP_C1 C1 46 13 DC 0F
  END
[143D]   BLOCK (exit -> @1511)
[1441]     AWAIT gameflag_252A
[1442]     GUARD rec_103A == 4024
[1447]     GUARD rec_1340 == 4060
[144C]     GUARD active_actor == Migrator.talk (related 40)
[1451]     ENDIF
[1452]     SAY "What's going on ? My Tina's in tears ..."  '[voice 6]
[146C]     SAY "It's not my fault... She didn't want to invite me to her wedding concert..."
[1490]     SAY "Okay.. Okay... I'll see what I can do..."  '[voice 7]
[14A8]     SAY "Do you have the ring, Commander ? The wedding band ? word_65535 teleport"  '[voice 2]
[14CC]     IF-BLOCK (exit -> @14EB)
[14CF]       GUARD concept == "teleport"
[14D2]       ENDIF
[14D3]       SAY "TELEPORT RING TO MIGRATOR"  '[skip 2]
[14E3]       OP_CD CD 30 00 DC 12 3A 04
[14EA]       CLEAR concept_alt
    END
[14EB]     SAY "Thank you, COMMANDER... Get comfortable... The show's about to start.."  '[voice 8, skip 2]
[1507]     OP_C1 C1 46 13 0C 10
[150C]     rec_0452 = 4108
  END
[1511]   BLOCK (exit -> @1777)
[1515]     AWAIT gameflag_252A
[1516]     GUARD rec_103A == 4024
[151B]     GUARD rec_1340 == 4108
[1520]     GUARD active_actor == Migrator.talk (related 40)
[1525]     ENDIF
[1526]     SAY "SILENCE IT'S STARTING...."  '[skip 1]
[1534]     LOADSTR "lpm6sc1.hnm"
[1542]     SAY "Ah at last ... It looks like it's showtime..."  '[skip 1]
[155C]     LOADSTR "bobb.hnm"
[1567]     SAY "Move over ... I wanna see too..."  '[skip 1]
[157D]     LOADSTR "hboc.hnm"
[1588]     SAY "Take it easy... We're not gonna fight over this..."
[15A2]     SAY "You think this is the end of the game, Cap'n Bob???"
[15C0]     SAY "Gosh darn it NO, Mister Honk... The Commander still hasn't found the BIG BANG..."  '[skip 1]
[15E4]     LOADSTR "bobb.hnm"
[15EF]     SAY ""  '[skip 1]
[15F7]     LOADSTR "lpl7sc1.hnm"
[1605]     SAY ""  '[skip 1]
[160D]     LOADSTR "lpm1sc1.hnm"
[161B]     SAY ""  '[skip 1]
[1623]     LOADSTR "lpm2sc1.hnm"
[1631]     SAY ""  '[skip 1]
[1639]     LOADSTR "lpm3sc1.hnm"
[1647]     SAY ""  '[skip 1]
[164F]     LOADSTR "lpm4sc1.hnm"
[165D]     SAY ""  '[skip 1]
[1665]     LOADSTR "lpm7sc1.hnm"
[1673]     SAY ""  '[skip 1]
[167B]     LOADSTR "lpm5sc1.hnm"
[1689]     SAY ""  '[skip 1]
[1691]     LOADSTR "lpm6sc1.hnm"
[169F]     SAY ""  '[skip 1]
[16A7]     LOADSTR "lpl7sc1.hnm"
[16B5]     SAY ""  '[skip 1]
[16BD]     LOADSTR "lpm1sc1.hnm"
[16CB]     SAY ""  '[skip 1]
[16D3]     LOADSTR "lpm2sc1.hnm"
[16E1]     SAY ""  '[skip 1]
[16E9]     LOADSTR "lpm3sc1.hnm"
[16F7]     SAY ""  '[skip 1]
[16FF]     LOADSTR "lpm4sc1.hnm"
[170D]     SAY ""  '[skip 1]
[1715]     LOADSTR "lpm7sc1.hnm"
[1723]     SAY ""  '[skip 1]
[172B]     LOADSTR "lpm5sc1.hnm"
[1739]     SAY ""  '[skip 1]
[1741]     LOADSTR "lpm6sc1.hnm"
[174F]     SAY ""  '[skip 1]
[1757]     LOADSTR "lpm7sc1.hnm"
[1765]     SAY ""  '[skip 1]
[176D]     LOADSTR "fin.hnm"
  END
[1777]   BLOCK (exit -> @181C)
[177B]     AWAIT gameflag_252A
[177C]     GUARD A1 == 0
[1783]     GUARD rec_103A == 3422
[1788]     GUARD active_actor == Izwalito.talk (related 40)
[178D]     ENDIF
[178E]     SAY "Izwalito happy see you again , friend ..."  '[voice 2]
[17A6]     SAY "Me get ready to leave , friend"  '[voice 0]
[17BC]     SAY "Me go big party with friends ..."  '[voice 3]
[17D2]     SAY "Sorry ... me not have much time ..."  '[voice 2]
[17EA]     SAY "Me in hurry , Bossanova impatient ..."  '[voice 5]
[1800]     SAY "Bye bye , friend Commander ..."  '[voice 5, skip 2]
[1814]     rec_05BA = 3884
[1819]     END PRESENTATION Izwalito.talk
  END
[181C]   BLOCK (exit -> @1829)
[1820]     ENDIF
[1821]     state[10] = 800
[1825]     POKE [0x181D] = 0
  END
[1829]   BLOCK (exit -> @1839)
[182D]     GUARD state[10] == 0
[182F]     ENDIF
[1830]     OP_C3 C3 4C 05 28 00
[1835]     POKE [0x182A] = 0
  END
[1839]   BLOCK (exit -> @194D)
[183D]     AWAIT presentation
[183E]     GUARD active_actor == Kran_Dobu.talk (related 40)
[1843]     ENDIF
[1844]     SAY "This is Kran Dobu , Space Knight of the Stars ..."
[1862]     SAY "Radio message to Ark ... Ha ! Ha ! You okay ? ..."
[1884]     SAY "Ha ! Ha ! I hear you're looking for the the Big Bang too ..."
[18AA]     SAY "I've pinpointed his ship , Commander ..."
[18C0]     SAY "That's where I'm headed , except I don't have the address ... Ha ! Ha ! ..."
[18EA]     SAY "We'll meet up there ... Ha ! Ha ! Ha ! ..."
[190A]     SAY "He's talking about the Big Bang ..."
[1920]     SAY "See you ... Ha ! Ha ! Ha !"
[193A]     SAY "Bye bye"  '[skip 2]
[1946]     POKE [0x183A] = 0
[194A]     END PRESENTATION Kran_Dobu.talk
  END
[194D]   BLOCK (exit -> @1A94)
[1951]     AWAIT gameflag_252A
[1952]     GUARD active_actor == Scruter_Mac.talk (related 40)
[1957]     ENDIF
[1958]     SAY "YOU IN FORBIDDEN ZONE ... ME GIVE WARNING , STRANGER"  '[voice 18]
[1974]     SAY "Me not know you ."  '[voice 17]
[1986]     SAY "It must be Scruter Mac's twin brother , Commander ... Took over his job ..."
[19AC]     SAY "You say code"  '[voice 16]
[19BA]     SAY "Hurry , stranger! word_65535 croolas galabar code pterra exxos"  '[voice 1]
[19D6]     IF-BLOCK (exit -> @1A1C)
[19D9]       GUARD concept == "code"
[19DC]       ENDIF
[19DD]       SAY "You say code ..."  '[voice 12]
[19ED]       SAY "You not stupid ... But me smarter than you ..."  '[voice 14]
[1A09]       SAY "You want see prisoner ..."  '[voice 7, skip 1]
[1A1B]       CLEAR concept_alt
    END
[1A1C]     IF-BLOCK (exit -> @1A77)
[1A1F]       GUARD NOT concept == "code"
[1A23]       ENDIF
[1A24]       SAY "You not say code ... Me not like ... Not like ..."  '[voice 9]
[1A44]       SAY "You meet my laser-pulper ..."  '[voice 7]
[1A56]       SAY "BYE BYE FRIEND ..."  '[voice 8, skip 3]
[1A66]       LOADSTR "explo3.hnm"
[1A73]       CLEAR concept_alt
[1A74]       END PRESENTATION Scruter_Mac.talk
    END
[1A77]     SAY "You know code , me open door ..."  '[voice 5, skip 1]
[1A8F]     OP_C1 C1 46 13 64 0A
  END
[1A94]   BLOCK (exit -> @1C93)
[1A98]     AWAIT gameflag_252A
[1A99]     GUARD D1 == 0
[1AA0]     GUARD active_actor == Eviscerator.talk (related 40)
[1AA5]     GUARD rec_103A == 2606
[1AAA]     ENDIF
[1AAB]     SAY "Me know you ... You give me mummy ... and CURSE ..."  '[voice 5]
[1ACB]     SAY "ME HATE YOU ..."  '[voice 4]
[1ADB]     SAY "WHAT YOU WANT STRANGER ?"  '[voice 7, skip 1]
[1AED]     rec_01F8 = 1
[1AF2]     IF-BLOCK (exit -> @1B1F)
[1AF5]       GUARD secret == 1
[1AFC]       ENDIF
[1AFD]       SAY "You like secrets ... Me tell secrets"  '[voice 6, skip 2]
[1B13]       rec_01F8 = 3943
[1B18]       secret = 0
    END
[1B1F]     IF-BLOCK (exit -> @1C11)
[1B22]       GUARD secret1 == 1
[1B29]       ENDIF
[1B2A]       SAY "SPLATCH be highly concentrated explosive . SPLATCH blow up jail ... LAUGH ... FOUL SWEAR"  '[voice 10]
[1B50]       SAY "You bring me SPLATCH , friend ... And me give you ..."  '[voice 8]
[1B70]       SAY "He's gonna offer us the treasure again , Commander ... Ol' Turkey-face Bob'll love this ..."
[1B98]       SAY "You go see friends at "Galaxian Bar" on planet MASTAGLUK ... Them help you ..."  '[voice 5]
[1BBE]       SAY "No... No ... That's enough, Commander ... This is not a never ending story ..."
[1BE4]       SAY "You understand ? Bye bye ... ME WAIT YOU ..."  '[voice 9, skip 3]
[1C00]       secret1 = 0
[1C07]       D1 = 1
[1C0E]       END PRESENTATION Eviscerator.talk
    END
[1C11]     IF-BLOCK (exit -> @1C40)
[1C14]       GUARD vbio == 0
[1C1B]       ENDIF
[1C1C]       SAY "Commander , if I'd had some BIONIUM , I could have helped you ...."
    END
[1C40]     IF-BLOCK (exit -> @1C6A)
[1C43]       GUARD vbio > 3
[1C4A]       ENDIF
[1C4B]       SAY "Commander , there's no percentage staying here ..."  '[skip 1]
[1C63]       vbio -= 3
    END
[1C6A]     SAY "Bye bye . You come back see me . Me like visits ... word_65535 bye_bye"  '[voice 6, skip 1]
[1C90]     END PRESENTATION Eviscerator.talk
  END
[1C93]   BLOCK (exit -> @1E07)
[1C97]     AWAIT gameflag_252A
[1C98]     GUARD D1 == 1
[1C9F]     GUARD rec_1340 == 2660
[1CA4]     GUARD active_actor == Eviscerator.talk (related 40)
[1CA9]     ENDIF
[1CAA]     SAY "You have splatch ???"  '[voice 8]
[1CBA]     SAY "Me big CROOLIS EVISCERATOR . Me want escape ... CRY ... CRY ... GNASH ..."  '[voice 1]
[1CE0]     SAY "CRY ... GNASH ..."  '[voice 1, skip 1]
[1CF0]     rec_01F8 = 1646
[1CF5]     IF-BLOCK (exit -> @1D35)
[1CF8]       GUARD rec_1290 == 40
[1CFD]       ENDIF
[1CFE]       SAY "Give him the splatch , Commander ... We need the treasure ..."
[1D1E]       SAY "TELEPORT SPLATCH TO EVISCERATOR"  '[skip 1]
[1D2E]       OP_CD CD 30 00 7C 12 B2 01
    END
[1D35]     IF-BLOCK (exit -> @1DAC)
[1D38]       GUARD rec_1290 == 434
[1D3D]       ENDIF
[1D3E]       SAY "You want know where be treasure ..."  '[voice 6]
[1D54]       SAY "Treasure be on planet TUMUL..."  '[voice 8]
[1D66]       SAY "Coordinates AX329 Tumulus constellation ..."  '[voice 5]
[1D78]       SAY "That sounded like a treasure location to me , Commander ... Way to go , champ !!!"  '[skip 2]
[1DA2]       D1 = 2
[1DA9]       END PRESENTATION Eviscerator.talk
    END
[1DAC]     IF-BLOCK (exit -> @1E07)
[1DAF]       GUARD NOT rec_1290 == 40
[1DB5]       GUARD NOT rec_1290 == 434
[1DBB]       ENDIF
[1DBC]       SAY "INSULT ... UGLY SWEAR ... GNASH ... GNASH ... UGLY LAUGH ..."  '[voice 10]
[1DDC]       SAY "ME HATE YOU ... INSULT ...."  '[voice 12]
[1DF0]       SAY "ME NOT SAY BYE BYE ..."  '[voice 11, skip 1]
[1E04]       END PRESENTATION Eviscerator.talk
    END
  END
[1E07]   BLOCK (exit -> @1EB8)
[1E0B]     AWAIT gameflag_252A
[1E0C]     GUARD rec_0602 == 3092
[1E11]     GUARD active_actor == Amigo.talk (related 40)
[1E16]     GUARD rec_103A == 3038
[1E1B]     GUARD M1 == 0
[1E22]     ENDIF
[1E23]     SAY "HALT HIC STRANGER !!!"  '[voice 3]
[1E33]     SAY "The Bar's hic ! shut ... Hic !"  '[voice 5]
[1E4B]     SAY "They've all gone ... Hic ... Just me left ..."  '[voice 6]
[1E67]     SAY "Commander... I feel sorry for this buzzard ..."
[1E7F]     SAY "Me thirsty ... Hic ..."  '[voice 6]
[1E91]     SAY "Me need hic drinky hic !!!"  '[voice 6]
[1EA5]     SAY "Bye hic bye ..."  '[voice 0, skip 1]
[1EB5]     END PRESENTATION Amigo.talk
  END
[1EB8]   BLOCK (exit -> @1FC0)
[1EBC]     AWAIT gameflag_274F
[1EBD]     GUARD active_actor == Yoko.talk (related 40)
[1EC2]     ENDIF
[1EC3]     SAY "Commander ... MAXXON and me would like to get back to our home on the planet RONDO ..."  '[voice 1]
[1EEF]     SAY "Can you drop us off there ? ..."  '[voice 3]
[1F07]     IF-BLOCK (exit -> @1F75)
[1F0A]       GUARD rec_103A == 2930
[1F0F]       ENDIF
[1F10]       SAY "Why don't we teleport them , Commander ... word_65535 teleport refuse"
[1F30]       IF-BLOCK (exit -> @1F56)
[1F33]         GUARD concept == "teleport"
[1F36]         ENDIF
[1F37]         SAY "TELEPORT YOKO AND MAXXON ON RONDO"  '[skip 3]
[1F4B]         rec_025A = 2930
[1F50]         rec_0572 = 2984
[1F55]         CLEAR concept_alt
      END
[1F56]       IF-BLOCK (exit -> @1F75)
[1F59]         GUARD concept == "refuse"
[1F5C]         ENDIF
[1F5D]         SAY "Whatever you say , Commander ..."  '[skip 2]
[1F71]         CLEAR concept_alt
[1F72]         END PRESENTATION Yoko.talk
      END
    END
[1F75]     IF-BLOCK (exit -> @1FAB)
[1F78]       GUARD NOT rec_103A == 2930
[1F7E]       GUARD vbio > 3
[1F85]       ENDIF
[1F86]       SAY "We'll need to get closer to RONDO to teleport them ..."  '[skip 1]
[1FA4]       vbio -= 3
    END
[1FAB]     SAY "We're waiting , Commander ..."  '[voice 1, skip 1]
[1FBD]     END PRESENTATION Yoko.talk
  END
[1FC0]   BLOCK (exit -> @20C4)
[1FC4]     AWAIT gameflag_274F
[1FC5]     GUARD active_actor == Maxxon.talk (related 40)
[1FCA]     ENDIF
[1FCB]     SAY "Commander ... Yoko and me would like to get back to our home on the planet RONDO ..."  '[voice 1]
[1FF7]     SAY "Can you drop us off there ? ..."  '[voice 3]
[200F]     IF-BLOCK (exit -> @2079)
[2012]       GUARD rec_103A == 2930
[2017]       ENDIF
[2018]       SAY "Why don't we teleport them , Commander... word_65535 teleport refuse"
[2036]       IF-BLOCK (exit -> @205C)
[2039]         GUARD concept == "teleport"
[203C]         ENDIF
[203D]         SAY "TELEPORT YOKO AND MAXXON TO RONDO"  '[skip 3]
[2051]         rec_0572 = 2984
[2056]         rec_025A = 2930
[205B]         CLEAR concept_alt
      END
[205C]       IF-BLOCK (exit -> @2079)
[205F]         GUARD concept == "refuse"
[2062]         ENDIF
[2063]         SAY "You're the commander , Commander..."  '[skip 2]
[2075]         CLEAR concept_alt
[2076]         END PRESENTATION Maxxon.talk
      END
    END
[2079]     IF-BLOCK (exit -> @20AF)
[207C]       GUARD NOT rec_103A == 2930
[2082]       GUARD vbio > 3
[2089]       ENDIF
[208A]       SAY "We'll need to get closer to RONDO to teleport them ..."  '[skip 1]
[20A8]       vbio -= 3
    END
[20AF]     SAY "We're waiting , Commander ..."  '[voice 1, skip 1]
[20C1]     END PRESENTATION Maxxon.talk
  END
[20C4]   BLOCK (exit -> @216D)
[20C8]     AWAIT gameflag_252A
[20C9]     GUARD rec_103A == 2930
[20CE]     GUARD active_actor == Yoko.talk (related 40)
[20D3]     ENDIF
[20D4]     SAY "Me happy be back on my planet , Commander ..."  '[voice 0]
[20F0]     SAY "Hello Mister Honk..."  '[voice 2]
[20FE]     SAY "Yo Mister Yoko... How's my main man ?"
[2116]     SAY "Good , Mister Honk ..."  '[voice 4]
[2128]     SAY "Ha ! Ha ! ha ! You gotta love that Yoko ..."
[2148]     SAY "See you soon , Commander ... word_65535 bye_bye"  '[voice 5, skip 3]
[2160]     rec_025A = 2984
[2165]     rec_0572 = 2930
[216A]     END PRESENTATION Yoko.talk
  END
[216D]   BLOCK (exit -> @2220)
[2171]     AWAIT gameflag_252A
[2172]     GUARD rec_103A == 2930
[2177]     GUARD active_actor == Maxxon.talk (related 40)
[217C]     ENDIF
[217D]     SAY "Hello , friend ..."  '[voice 9]
[218D]     SAY "Yoko went to a party ..."  '[voice 2]
[21A1]     SAY "He's young ... he needs his fun ... Heh heh ..."  '[voice 6]
[21BF]     SAY "What can I do for you , Commander ?"  '[voice 7]
[21D9]     SAY "All these adventures have tired me out ..."  '[voice 4]
[21F1]     SAY "I'm going to have a vacation here ..."  '[voice 7]
[2209]     SAY "See you soon , Commander ..."  '[skip 1]
[221D]     END PRESENTATION Maxxon.talk
  END
[2220]   BLOCK (exit -> @2364)
[2224]     AWAIT gameflag_252A
[2225]     GUARD rec_103A == 3854
[222A]     GUARD active_actor == Daddy_Gluxx.talk (related 40)
[222F]     ENDIF
[2230]     SAY "Ho ! The visitor from far away in space and time ..."  '[voice 1]
[2250]     SAY "You've come back , stranger ..."  '[voice 2]
[2264]     SAY "My children , Gelatine , Rubber , Gooseberry and Latex thank you..."  '[voice 3]
[2284]     SAY "Thanks to you and Inspector Jerry Khan, they were saved from the claws of that awful doctor ..."  '[voice 4]
[22B0]     SAY "Thank you again ..."  '[voice 5]
[22C0]     SAY "Don't you just love it when they're grateful , Commander ..."
[22DE]     SAY "We're going to take a few days break ... My kids want to see the BIG BANG ..."  '[voice 0]
[230A]     SAY "After what they've been through ,"  '[voice 4]
[231E]     SAY "I can't refuse them anything ... Ha ! Ha ! Ha ..."  '[voice 1]
[233E]     SAY "Bye bye , Commander ... You'll always be welcome here ..."  '[skip 2]
[235C]     rec_049A = 3884
[2361]     END PRESENTATION Daddy_Gluxx.talk
  END
[2364]   BLOCK (exit -> @23ED)
[2368]     GUARD rec_103A == 2408
[236D]     GUARD active_actor == Bug_Deluxe.talk (related 40)
[2372]     ENDIF
[2373]     SAY "Hi there , Consumer Comrade . The VENUSIA SUPRAMARKET is shut today ..."  '[voice 1]
[2395]     SAY "It's a vacation for everybody ..."  '[voice 2]
[23A9]     SAY "Come back another time ... Oh and , Consumer Comrade ... Remember to bring your creds"  '[voice 3]
[23D1]     SAY "Bye bye , Consumer Comrade ..."  '[voice 0, skip 2]
[23E5]     rec_00AA = 4024
[23EA]     END PRESENTATION Bug_Deluxe.talk
  END
[23ED]   BLOCK (exit -> @26CD)
[23F1]     AWAIT gameflag_274F
[23F2]     GUARD F1 == 0
[23F9]     GUARD active_actor == Bronko.talk (related 40)
[23FE]     ENDIF
[23FF]     SAY "Everything's okay , Commander ..."  '[voice 2]
[2411]     SAY "Don't hesitate if you need me ..."  '[voice 3]
[2427]     IF-BLOCK (exit -> @268E)
[242A]       GUARD (airport.talk & 0x2) != 0
[242F]       ENDIF
[2430]       SAY "Teleport me to the BIG BANG , Commander ... I want to party ! Gee , I love weddings ..."  '[voice 5]
[2460]       SAY "Oh ! You're not going to leave me here alone , are you , Mister Bronko ? I'm a sucker for weddings too ..."
[2498]       SAY "I'll feel so loneley if you go ..."
[24B0]       SAY "Well , I sure don't want to to make you unhappy , Mister Honk ... That'll make me unhappy ..."  '[voice 6]
[24E0]       SAY "You're such an understanding being ..."  '[voice 7]
[24F4]       SAY "Oh ! And you , Mister Bronko, you have so much to offer ... Your cooking is out of this world ..."
[2528]       SAY "IF YOU GO ... I'M COMING WITH YOU ..."
[2542]       SAY "What's going on now ? HONK ! Your sentimentality makes me sick !!!"  '[skip 1]
[2564]       LOADSTR "bobb.hnm"
[256F]       SAY "Boo ! Hoo ! Sniff ... I'm unhappy ... Mister Bronko wants to party at the BIG BANG ..."
[259D]       SAY "Honk ! Can it ... I think I'm going to switch you off !!!"  '[skip 1]
[25C1]       LOADSTR "bobb.hnm"
[25CC]       SAY "NO ! No ! Please don't do that , Cap'n Bob ..."
[25EC]       SAY "Okay okay ... I'll stay here ... I don't really like to party anyhow ..."  '[voice 4]
[2612]       SAY "Oh , Mister Bronko ... Such a lesson in bigheartedness ... CRY ... WEEP ..."
[2638]       SAY "Commander ... Your computer's losing it ... Something needs to be done !!!"  '[skip 1]
[265A]       LOADSTR "bobb.hnm"
[2665]       SAY "Well , if you'll excuse me ... I've got a murffalo in the oven ..."  '[voice 6, skip 1]
[268B]       END PRESENTATION Bronko.talk
    END
[268E]     SAY "Oh Mister Bronko ... You're looking well, I must say !!!."  '[voice 5]
[26AC]     SAY "See you soon , Commander ..."  '[voice 5]
[26C0]     SAY "..."  '[skip 1]
[26CA]     END PRESENTATION Bronko.talk
  END
[26CD]   BLOCK (exit -> @2765)
[26D1]     AWAIT gameflag_252A
[26D2]     GUARD active_actor == Emasculator.talk (related 40)
[26D7]     GUARD J1 == 0
[26DE]     ENDIF
[26DF]     SAY "What you want , stranger ? You want murffalo meat ?"  '[voice 1]
[26FD]     SAY "Me not can sell murffalo ... Me shut store ..."  '[voice 2]
[2719]     SAY "Me go party ..."  '[voice 4]
[2729]     SAY "Me pack bags ... You not waste my time ..."  '[voice 4]
[2745]     SAY "BYE BYE ..."  '[voice 5]
[2753]     SAY "stop"  '[skip 2]
[275D]     rec_088A = 3884
[2762]     END PRESENTATION Emasculator.talk
  END
[2765]   BLOCK (exit -> @27F5)
[2769]     AWAIT gameflag_274F
[276A]     GUARD rec_025A == 65535
[276F]     GUARD active_actor == Hom.talk (related 40)
[2774]     ENDIF
[2775]     SAY "Ha ! ha ! Big Bang ... Black holes ... You're out of your mind ..."  '[voice 1]
[279D]     SAY "Commander ... Yoko and Maxxon would like to get back home on the planet RONDO ..."  '[voice 5]
[27C5]     SAY "They're impatient ..."  '[voice 6, skip 1]
[27D3]     rec_0240 = 1
[27D8]     SAY "Bye bye... Me sleep in star dust... word_65535 bye_bye"  '[voice 7, skip 1]
[27F2]     END PRESENTATION Hom.talk
  END
[27F5]   BLOCK (exit -> @2922)
[27F9]     AWAIT gameflag_274F
[27FA]     GUARD NOT rec_025A == 65535
[2800]     GUARD active_actor == Hom.talk (related 40)
[2805]     ENDIF
[2806]     SAY "Ha ! ha ! Big Bang ... Black holes ... You're out of your mind ..."  '[voice 1]
[282E]     SAY "Don't laugh , Mister Hom ... It's not a joke ..."
[284C]     SAY "Wrong , my friend . It's a big joke ... I'm leaving , Commander ..."  '[voice 5]
[2872]     SAY "I'm going to vanish into the cybernight ... And pick me up some star dust ..."  '[voice 6]
[289A]     SAY "I'm going to melt into the azure cyberworld ..."  '[voice 7]
[28B4]     SAY "I'll travel faster than the fastest ship ..."  '[voice 6]
[28CC]     SAY "And blur into a cybertrip ..."  '[voice 5]
[28E0]     SAY "Wasn't that beautiful , Commander ... We just experienced poetic creation ..."
[2900]     SAY "Bye bye , my friends ... BYE BYE ..."  '[voice 4, skip 2]
[291A]     rec_0212 = 2870
[291F]     END PRESENTATION Hom.talk
  END
[2922]   BLOCK (exit -> @2B1D)
[2926]     AWAIT gameflag_252A
[2927]     GUARD NOT rec_10F8 == 40
[292D]     GUARD active_actor == Hom.talk (related 40)
[2932]     ENDIF
[2933]     SAY "Welcome stranger ... You be big traveller ..."  '[voice 2]
[294B]     SAY "Me HOM. Me big Tubular Brain . Me welcome you , Commander ..."  '[voice 2]
[296D]     SAY "Me catch your print , friend ... Me read truth in your prints ..."  '[voice 2]
[2991]     SAY "You click very very hard , friend . Me catch print on mouse ... word_65535 click"  '[voice 3]
[29BB]     IF-BLOCK (exit -> @29F0)
[29BE]       GUARD concept == "click"
[29C1]       ENDIF
[29C2]       SAY "Ahh ! Me catch print ... Ohh ! Nice print ..."  '[voice 3, skip 2]
[29E0]       LOADSTR "scandoig.hnm"
[29EF]       CLEAR concept_alt
    END
[29F0]     SAY "Ohh ...Me see you look for Big Bang ..."  '[voice 4]
[2A0A]     SAY "You be with GUILD OF MEMBERS ... Me help you ..."  '[voice 9]
[2A28]     SAY "GUILD OF MEMBERS be big cybernetic Organization ..."  '[voice 10]
[2A40]     SAY "Me hear of Big Bang but me forget ..."  '[voice 3]
[2A5A]     SAY "Me find out and me tell you where be Big Bang . But you take exam ..."  '[voice 5]
[2A84]     SAY "You go planet CYBEROCK ..."  '[voice 4]
[2A96]     SAY "You study cyberknowledge , friend ..."  '[voice 4]
[2AAA]     SAY "Cyberock be in Cyberius system , coordinates X234 Y546 ..."  '[voice 5, skip 1]
[2AC6]     rec_0D42 |= 0x2
[2ACB]     SAY "We already know that , Commander ... Old Tube-head's going gaga ..."
[2AEB]     SAY "Me wait here with U.R.O.U.T ..."  '[voice 6]
[2AFF]     SAY "Bye bye friend ..."  '[skip 3]
[2B0F]     POKE [0x27F6] = 0
[2B13]     G1 = 1
[2B1A]     END PRESENTATION Hom.talk
  END
[2B1D]   BLOCK (exit -> @2D05)
[2B21]     AWAIT gameflag_252A
[2B22]     GUARD G1 == 1
[2B29]     GUARD active_actor == Hom.talk (related 40)
[2B2E]     ENDIF
[2B2F]     SAY "Welcome stranger ... You be big traveller ..."  '[voice 2]
[2B47]     SAY "Me HOM. Me big Tubular Brain . Me welcome you , Commander ..."  '[voice 2]
[2B69]     SAY "Me catch your print , friend ... Me read truth in your prints ..."  '[voice 2]
[2B8D]     SAY "Commander , he's getting up my nose with his print ..."
[2BAB]     SAY "You click very very hard , friend . Me catch print on mouse ... word_65535 click"  '[voice 3]
[2BD5]     IF-BLOCK (exit -> @2C0A)
[2BD8]       GUARD concept == "click"
[2BDB]       ENDIF
[2BDC]       SAY "Ahh ! Me catch print ... Ohh ! Nice print ..."  '[voice 3, skip 2]
[2BFA]       LOADSTR "scandoig.hnm"
[2C09]       CLEAR concept_alt
    END
[2C0A]     IF-BLOCK (exit -> @2C59)
[2C0D]       GUARD NOT rec_10F8 == 40
[2C13]       ENDIF
[2C14]       SAY "You must take U.R.O.U.T exam on planet CYBEROCK ..."
[2C2E]       SAY "Me wait for you ..."
[2C40]       SAY "Bye bye , friend ... word_65535 bye_bye"  '[skip 1]
[2C56]       END PRESENTATION Hom.talk
    END
[2C59]     IF-BLOCK (exit -> @2D05)
[2C5C]       GUARD rec_10F8 == 40
[2C61]       ENDIF
[2C62]       SAY "Bravo Commander . You be talented ..."
[2C78]       SAY "Me have good news for you ... Me know where be Big Bang ..."
[2C9C]       SAY "Big Bang be at coordinates X543 Y769..."  '[skip 1]
[2CB2]       airport.talk |= 0x2
[2CB7]       SAY "Bye bye , friend ... ME see you at Big Bang .... Ha Ha Ha ..."
[2CDF]       SAY "Well , how about that !"
[2CF3]       SAY "..."  '[skip 2]
[2CFD]       rec_0212 = 3884
[2D02]       END PRESENTATION Hom.talk
    END
  END
[2D05]   BLOCK (exit -> @2D59)
[2D09]     AWAIT gameflag_252A
[2D0A]     GUARD R1 == 0
[2D11]     GUARD active_actor == Cyberquizz.talk (related 40)
[2D16]     GUARD NOT rec_10F8 == 650
[2D1C]     GUARD rec_103A == 3392
[2D21]     ENDIF
[2D22]     SAY "Bye bye , student Commander Blood . You are a member of the "GUILD OF MEMBERS" ..."  '[voice 1]
[2D4C]     SAY "..."  '[voice 2, skip 1]
[2D56]     END PRESENTATION Cyberquizz.talk
  END
[2D59]   BLOCK (exit -> @2E06)
[2D5D]     AWAIT gameflag_252A
[2D5E]     GUARD R1 == 0
[2D65]     GUARD active_actor == Cyberquizz.talk (related 40)
[2D6A]     GUARD rec_103A == 3392
[2D6F]     ENDIF
[2D70]     POKE [0x3056] = 1
[2D74]     POKE [0x30FF] = 1
[2D78]     POKE [0x31A6] = 1
[2D7C]     POKE [0x3229] = 1
[2D80]     POKE [0x32C2] = 1
[2D84]     POKE [0x3351] = 1
[2D88]     POKE [0x33D8] = 1
[2D8C]     POKE [0x3465] = 1
[2D90]     POKE [0x34E4] = 1
[2D94]     POKE [0x3567] = 1
[2D98]     POKE [0x35FC] = 1
[2D9C]     POKE [0x3670] = 1
[2DA0]     POKE [0x36F7] = 1
[2DA4]     POKE [0x3778] = 1
[2DA8]     POKE [0x3805] = 1
[2DAC]     POKE [0x389A] = 1
[2DB0]     POKE [0x3921] = 1
[2DB4]     POKE [0x39BA] = 1
[2DB8]     POKE [0x3A3F] = 1
[2DBC]     POKE [0x3AD0] = 1
[2DC0]     POKE [0x3B69] = 1
[2DC4]     POKE [0x3BEC] = 1
[2DC8]     POKE [0x3C6D] = 1
[2DCC]     POKE [0x3CF6] = 1
[2DD0]     POKE [0x3D71] = 1
[2DD4]     POKE [0x3DF6] = 1
[2DD8]     POKE [0x3E85] = 1
[2DDC]     POKE [0x3F0E] = 1
[2DE0]     POKE [0x3F9F] = 1
[2DE4]     POKE [0x402E] = 1
[2DE8]     POKE [0x40B5] = 1
[2DEC]     POKE [0x4140] = 1
[2DF0]     quest = 0
[2DF7]     Bof = 0
[2DFE]     POKE [0x2E07] = 1
[2E02]     POKE [0x2D5A] = 0
  END
[2E06]   GOTO @2ECF
[2E0A]   AWAIT gameflag_252A
[2E0B]   GUARD R1 == 0
[2E12]   GUARD active_actor == Cyberquizz.talk (related 40)
[2E17]   GUARD rec_103A == 3392
[2E1C]   ENDIF
[2E1D]   SAY "Student Commander Blood ... This is a big day in your life ..."  '[voice 1]
[2E3F]   SAY "Holy harpoons ! That is hideous , Commander..."
[2E57]   SAY "I'm going to be your U.R.O.U.T. examiner"  '[voice 2]
[2E6D]   SAY "Universal Revolutionary Omniscient Ubiquity Test"  '[voice 3]
[2E7F]   SAY "Stupid animal ! At your age , I already had my U.R.O.U.T.F.O.R.T.H.E.C.O.U.N.T."  '[voice 3]
[2E9F]   SAY "Don't let him scare you, Commander. He's just jealous..."
[2EB9]   SAY "Here are the questions. Are you ready?"  '[voice 4, skip 1]
[2ECF]   BLOCK (exit -> @2FBD)
[2ED3]     AWAIT gameflag_252A
[2ED4]     GUARD active_actor == Cyberquizz.talk (related 40)
[2ED9]     GUARD Bof == 5
[2EE0]     ENDIF
[2EE1]     SAY "Believe it or not ... You have passed your U.R.O.U.T."  '[voice 1]
[2EFD]     SAY "YESSS!!!! I KNEW YOU COULD DO IT COMMANDER..."  '[skip 1]
[2F15]     LOADSTR "cadeaux.hnm"
[2F23]     SAY "I present you with your magnificent diploma..."  '[voice 1]
[2F39]     SAY "TELEPORT UROUT to Ark word_65535 teleport"
[2F4F]     IF-BLOCK (exit -> @2F6E)
[2F52]       GUARD concept == "teleport"
[2F55]       ENDIF
[2F56]       SAY "TELEPORTING DIPLOMA TO ARK"  '[skip 2]
[2F66]       OP_CD CD C4 02 E4 10 28 00
[2F6D]       CLEAR concept_alt
    END
[2F6E]     SAY "Never forget your happy student days. Be true to your school!"  '[voice 3]
[2F8C]     SAY "See you soon, Commander..."  '[voice 2]
[2F9C]     SAY "stop"  '[skip 4]
[2FA6]     Bof = 0
[2FAD]     quest = 0
[2FB4]     rec_0D42 &= !0x2
[2FBA]     END PRESENTATION Cyberquizz.talk
  END
[2FBD]   BLOCK (exit -> @3055)
[2FC1]     AWAIT gameflag_252A
[2FC2]     GUARD active_actor == Cyberquizz.talk (related 40)
[2FC7]     GUARD quest == 32
[2FCE]     GUARD Bof < 10
[2FD5]     ENDIF
[2FD6]     SAY "You failed!"  '[voice 10]
[2FE2]     SAY "I knew this would happen... I feel pity for you , Commander..."
[3002]     SAY "May your liver turn into oxidized mercury sauce!! How could you do this??..."  '[voice 9]
[3024]     SAY "I didn't deserve this humiliation...."
[3036]     SAY "stop"  '[skip 4]
[3040]     POKE [0x2D5A] = 1
[3044]     quest = 0
[304B]     Bof = 0
[3052]     END PRESENTATION Cyberquizz.talk
  END
[3055]   BLOCK (exit -> @30FE)
[3059]     AWAIT gameflag_252A
[305A]     GUARD active_actor == Cyberquizz.talk (related 40)
[305F]     ENDIF
[3060]     SAY "Who said "It takes thirteen murffalos to reproduce but soon it'll take fourteen? word_65535 Jeph_DUlikan Yolk Kran_Dobu Tina_Burner"  '[voice 9]
[308E]     IF-BLOCK (exit -> @30C2)
[3091]       GUARD concept == "Yolk"
[3094]       ENDIF
[3095]       SAY "The Great Yolk, of course. You've been boning up , Commander..."  '[voice 2, skip 3]
[30B3]       Bof += 1
[30BA]       quest += 1
[30C1]       CLEAR concept_alt
    END
[30C2]     IF-BLOCK (exit -> @30EE)
[30C5]       GUARD NOT concept == "Yolk"
[30C9]       ENDIF
[30CA]       SAY "History's not your speciality. Maybe you're not ready for this"  '[voice 5, skip 2]
[30E6]       quest += 1
[30ED]       CLEAR concept_alt
    END
[30EE]     SAY "Next question:"  '[voice 3, skip 1]
[30FA]     POKE [0x3056] = 0
  END
[30FE]   BLOCK (exit -> @31A5)
[3102]     AWAIT gameflag_252A
[3103]     GUARD active_actor == Cyberquizz.talk (related 40)
[3108]     ENDIF
[3109]     SAY "Which great Slimer is buried on the planet Vista? word_65535 gluxx gelati gumy yolk"  '[voice 6]
[312F]     IF-BLOCK (exit -> @3163)
[3132]       GUARD concept == "yolk"
[3135]       ENDIF
[3136]       SAY "Yes indeed , the incomparably Great Yolk... Hare Yolk Hare Hare..."  '[voice 7, skip 3]
[3154]       Bof += 1
[315B]       quest += 1
[3162]       CLEAR concept_alt
    END
[3163]     IF-BLOCK (exit -> @3195)
[3166]       GUARD NOT concept == "yolk"
[316A]       ENDIF
[316B]       SAY "On Vista , I said . Get those ondoyants out of your brain..."  '[voice 8, skip 2]
[318D]       quest += 1
[3194]       CLEAR concept_alt
    END
[3195]     SAY "Next question:"  '[voice 3, skip 1]
[31A1]     POKE [0x30FF] = 0
  END
[31A5]   BLOCK (exit -> @3228)
[31A9]     AWAIT gameflag_252A
[31AA]     GUARD active_actor == Cyberquizz.talk (related 40)
[31AF]     ENDIF
[31B0]     SAY "Concentrate now..."  '[voice 5]
[31BC]     SAY "Name a stuttering Croolis? word_65535 Cybernator Rotator Emasculator Outrageor Lord_Krater"  '[voice 1]
[31DA]     IF-BLOCK (exit -> @31FE)
[31DD]       GUARD concept == "Rotator"
[31E0]       ENDIF
[31E1]       SAY "That's right ..."  '[voice 4, skip 3]
[31EF]       Bof += 1
[31F6]       quest += 1
[31FD]       CLEAR concept_alt
    END
[31FE]     IF-BLOCK (exit -> @3218)
[3201]       GUARD NOT concept == "Rotator"
[3205]       ENDIF
[3206]       SAY "..."  '[voice 3, skip 2]
[3210]       quest += 1
[3217]       CLEAR concept_alt
    END
[3218]     SAY "Next question:"  '[voice 3, skip 1]
[3224]     POKE [0x31A6] = 0
  END
[3228]   BLOCK (exit -> @32C1)
[322C]     AWAIT gameflag_252A
[322D]     GUARD active_actor == Cyberquizz.talk (related 40)
[3232]     ENDIF
[3233]     SAY "What was king Betakam IV's planet of origin? word_65535 Sat Vistar Ron Golgos Pterra Attrox"  '[voice 0]
[325B]     IF-BLOCK (exit -> @327D)
[325E]       GUARD concept == "Attrox"
[3261]       ENDIF
[3262]       SAY "Excellent answer..."  '[voice 9, skip 3]
[326E]       Bof += 1
[3275]       quest += 1
[327C]       CLEAR concept_alt
    END
[327D]     IF-BLOCK (exit -> @32B1)
[3280]       GUARD NOT concept == "Attrox"
[3284]       ENDIF
[3285]       SAY "For goodness sakes ! It starts with an A and ends with an X..."  '[voice 8, skip 2]
[32A9]       quest += 1
[32B0]       CLEAR concept_alt
    END
[32B1]     SAY "Next question:"  '[voice 3, skip 1]
[32BD]     POKE [0x3229] = 0
  END
[32C1]   BLOCK (exit -> @3350)
[32C5]     AWAIT gameflag_252A
[32C6]     GUARD active_actor == Cyberquizz.talk (related 40)
[32CB]     ENDIF
[32CC]     SAY "Which Trump has a friend called Trompette? word_65535 eviscerator Fifi beauregard tromp_la_mort otto_von_smile"  '[voice 7]
[32F0]     IF-BLOCK (exit -> @331A)
[32F3]       GUARD concept == "Fifi"
[32F6]       ENDIF
[32F7]       SAY "Fifi, that's the right answer ..."  '[voice 5, skip 3]
[330B]       Bof += 1
[3312]       quest += 1
[3319]       CLEAR concept_alt
    END
[331A]     IF-BLOCK (exit -> @3340)
[331D]       GUARD NOT concept == "Fifi"
[3321]       ENDIF
[3322]       SAY "For goodness sakes! Get a grip guy..."  '[voice 6, skip 2]
[3338]       quest += 1
[333F]       CLEAR concept_alt
    END
[3340]     SAY "Next question:"  '[voice 3, skip 1]
[334C]     POKE [0x32C2] = 0
  END
[3350]   BLOCK (exit -> @33D7)
[3354]     AWAIT gameflag_252A
[3355]     GUARD active_actor == Cyberquizz.talk (related 40)
[335A]     ENDIF
[335B]     SAY "The Great Yolk is a...? word_65535 slimer izwal croolis migrax robot"  '[voice 4]
[337B]     IF-BLOCK (exit -> @339F)
[337E]       GUARD concept == "slimer"
[3381]       ENDIF
[3382]       SAY "Yes, a Slimer..."  '[voice 3, skip 3]
[3390]       Bof += 1
[3397]       quest += 1
[339E]       CLEAR concept_alt
    END
[339F]     IF-BLOCK (exit -> @33C7)
[33A2]       GUARD NOT concept == "slimer"
[33A6]       ENDIF
[33A7]       SAY "The Great Yolk? That was a mediocre effort"  '[voice 2, skip 2]
[33BF]       quest += 1
[33C6]       CLEAR concept_alt
    END
[33C7]     SAY "Next question:"  '[voice 3, skip 1]
[33D3]     POKE [0x3351] = 0
  END
[33D7]   BLOCK (exit -> @3464)
[33DB]     AWAIT gameflag_252A
[33DC]     GUARD active_actor == Cyberquizz.talk (related 40)
[33E1]     ENDIF
[33E2]     SAY "Who said: "Commander, Honk keeps scanning me with his equipment"? word_65535 eviscerator emasculator lord_raptor tina_burner"  '[voice 1]
[340A]     IF-BLOCK (exit -> @3432)
[340D]       GUARD concept == "tina_burner"
[3410]       ENDIF
[3411]       SAY "Quite right. In the cryobox..."  '[voice 2, skip 3]
[3423]       Bof += 1
[342A]       quest += 1
[3431]       CLEAR concept_alt
    END
[3432]     IF-BLOCK (exit -> @3454)
[3435]       GUARD NOT concept == "tina_burner"
[3439]       ENDIF
[343A]       SAY "Don't you have a radio?"  '[voice 3, skip 2]
[344C]       quest += 1
[3453]       CLEAR concept_alt
    END
[3454]     SAY "Next question:"  '[voice 3, skip 1]
[3460]     POKE [0x33D8] = 0
  END
[3464]   BLOCK (exit -> @34E3)
[3468]     AWAIT gameflag_252A
[3469]     GUARD active_actor == Cyberquizz.talk (related 40)
[346E]     ENDIF
[346F]     SAY "Jeph d'Ulikan defeated? word_65535 pagos_tagos black_larsen hom lord_krater"  '[voice 4]
[3489]     IF-BLOCK (exit -> @34B1)
[348C]       GUARD concept == "black_larsen"
[348F]       ENDIF
[3490]       SAY "The Black Larsen Hordes, yes..."  '[voice 5, skip 3]
[34A2]       Bof += 1
[34A9]       quest += 1
[34B0]       CLEAR concept_alt
    END
[34B1]     IF-BLOCK (exit -> @34D3)
[34B4]       GUARD NOT concept == "black_larsen"
[34B8]       ENDIF
[34B9]       SAY "No , no and NO..."  '[voice 6, skip 2]
[34CB]       quest += 1
[34D2]       CLEAR concept_alt
    END
[34D3]     SAY "Next question:"  '[voice 3, skip 1]
[34DF]     POKE [0x3465] = 0
  END
[34E3]   BLOCK (exit -> @3566)
[34E7]     AWAIT gameflag_252A
[34E8]     GUARD active_actor == Cyberquizz.talk (related 40)
[34ED]     ENDIF
[34EE]     SAY "Who said: "One plus one equals three"? word_65535 lord_krater metro_paul yolk"  '[voice 7]
[350E]     IF-BLOCK (exit -> @3532)
[3511]       GUARD concept == "yolk"
[3514]       ENDIF
[3515]       SAY "The incredible Yolk..."  '[voice 8, skip 3]
[3523]       Bof += 1
[352A]       quest += 1
[3531]       CLEAR concept_alt
    END
[3532]     IF-BLOCK (exit -> @3556)
[3535]       GUARD NOT concept == "yolk"
[3539]       ENDIF
[353A]       SAY "You're not up on your classics..."  '[voice 9, skip 2]
[354E]       quest += 1
[3555]       CLEAR concept_alt
    END
[3556]     SAY "Next question:"  '[voice 3, skip 1]
[3562]     POKE [0x34E4] = 0
  END
[3566]   BLOCK (exit -> @35FB)
[356A]     AWAIT gameflag_252A
[356B]     GUARD active_actor == Cyberquizz.talk (related 40)
[3570]     ENDIF
[3571]     SAY "How many murffalos does it take to reproduce? word_65535 two four six eight thirteen"  '[voice 8]
[3597]     IF-BLOCK (exit -> @35C7)
[359A]       GUARD concept == "thirteen"
[359D]       ENDIF
[359E]       SAY "One in the middle and twelve all around. Good..."  '[voice 7, skip 3]
[35B8]       Bof += 1
[35BF]       quest += 1
[35C6]       CLEAR concept_alt
    END
[35C7]     IF-BLOCK (exit -> @35EB)
[35CA]       GUARD NOT concept == "thirteen"
[35CE]       ENDIF
[35CF]       SAY "Murffalos? That was the easy question..."  '[voice 6, skip 2]
[35E3]       quest += 1
[35EA]       CLEAR concept_alt
    END
[35EB]     SAY "Next question:"  '[voice 3, skip 1]
[35F7]     POKE [0x3567] = 0
  END
[35FB]   BLOCK (exit -> @366F)
[35FF]     AWAIT gameflag_252A
[3600]     GUARD active_actor == Cyberquizz.talk (related 40)
[3605]     ENDIF
[3606]     SAY "Name a paralleloped planet word_65535 qx20 tumul vista ebony mastachok"  '[voice 5]
[3624]     IF-BLOCK (exit -> @3643)
[3627]       GUARD concept == "vista"
[362A]       ENDIF
[362B]       SAY "Paralleloped? Try contact lenses..."  '[voice 4, skip 2]
[363B]       quest += 1
[3642]       CLEAR concept_alt
    END
[3643]     IF-BLOCK (exit -> @365F)
[3646]       GUARD NOT concept == "vista"
[364A]       ENDIF
[364B]       SAY "You're hopeless...."  '[voice 3, skip 2]
[3657]       quest += 1
[365E]       CLEAR concept_alt
    END
[365F]     SAY "Next question:"  '[voice 3, skip 1]
[366B]     POKE [0x35FC] = 0
  END
[366F]   BLOCK (exit -> @36F6)
[3673]     AWAIT gameflag_252A
[3674]     GUARD active_actor == Cyberquizz.talk (related 40)
[3679]     ENDIF
[367A]     SAY "Buggol democracy was invented by? word_65535 strum Bob_Morlock cyborg_1er exxos"  '[voice 2]
[3698]     IF-BLOCK (exit -> @36C8)
[369B]       GUARD concept == "cyborg_1er"
[369E]       ENDIF
[369F]       SAY "A bit of a history buff , eh ?..."  '[voice 1, skip 3]
[36B9]       Bof += 1
[36C0]       quest += 1
[36C7]       CLEAR concept_alt
    END
[36C8]     IF-BLOCK (exit -> @36E6)
[36CB]       GUARD NOT concept == "cyborg_1er"
[36CF]       ENDIF
[36D0]       SAY "No! No! Commander..."  '[voice 0, skip 2]
[36DE]       quest += 1
[36E5]       CLEAR concept_alt
    END
[36E6]     SAY "Next question:"  '[voice 3, skip 1]
[36F2]     POKE [0x3670] = 0
  END
[36F6]   BLOCK (exit -> @3777)
[36FA]     AWAIT gameflag_252A
[36FB]     GUARD active_actor == Cyberquizz.talk (related 40)
[3700]     ENDIF
[3701]     SAY "What do Tubular Brains eat? word_65535 klakos plastok radium hay murffalos"  '[voice 1]
[3721]     IF-BLOCK (exit -> @3743)
[3724]       GUARD concept == "plastok"
[3727]       ENDIF
[3728]       SAY "Protein-enriched Plastok."  '[voice 2, skip 3]
[3734]       Bof += 1
[373B]       quest += 1
[3742]       CLEAR concept_alt
    END
[3743]     IF-BLOCK (exit -> @3767)
[3746]       GUARD NOT concept == "plastok"
[374A]       ENDIF
[374B]       SAY "You got something against Tubulars ?"  '[voice 3, skip 2]
[375F]       quest += 1
[3766]       CLEAR concept_alt
    END
[3767]     SAY "Next question:"  '[voice 3, skip 1]
[3773]     POKE [0x36F7] = 0
  END
[3777]   BLOCK (exit -> @3804)
[377B]     AWAIT gameflag_252A
[377C]     GUARD active_actor == Cyberquizz.talk (related 40)
[3781]     ENDIF
[3782]     SAY "Name a planet with rings word_65535 erazor rondo bonus pterra"  '[voice 4]
[37A0]     IF-BLOCK (exit -> @37CE)
[37A3]       GUARD concept == "bonus"
[37A6]       ENDIF
[37A7]       SAY "Bonus has the finest rings in the universe."  '[voice 5, skip 3]
[37BF]       Bof += 1
[37C6]       quest += 1
[37CD]       CLEAR concept_alt
    END
[37CE]     IF-BLOCK (exit -> @37F4)
[37D1]       GUARD NOT concept == "bonus"
[37D5]       ENDIF
[37D6]       SAY "Rings... You know , those round things..."  '[voice 6, skip 2]
[37EC]       quest += 1
[37F3]       CLEAR concept_alt
    END
[37F4]     SAY "Next question:"  '[voice 3, skip 1]
[3800]     POKE [0x3778] = 0
  END
[3804]   BLOCK (exit -> @3899)
[3808]     AWAIT gameflag_252A
[3809]     GUARD active_actor == Cyberquizz.talk (related 40)
[380E]     ENDIF
[380F]     SAY "How do Scorps reproduce? word_65535 vivisection parthenogenesis mimicry chance"  '[voice 7]
[382B]     IF-BLOCK (exit -> @385F)
[382E]       GUARD concept == "parthenogenesis"
[3831]       ENDIF
[3832]       SAY "Yes... Just cut 'em up and you've got a family tree."  '[voice 8, skip 3]
[3850]       Bof += 1
[3857]       quest += 1
[385E]       CLEAR concept_alt
    END
[385F]     IF-BLOCK (exit -> @3889)
[3862]       GUARD NOT concept == "parthenogenesis"
[3866]       ENDIF
[3867]       SAY "You should open a dictionary from time to time"  '[voice 9, skip 2]
[3881]       quest += 1
[3888]       CLEAR concept_alt
    END
[3889]     SAY "Next question:"  '[voice 3, skip 1]
[3895]     POKE [0x3805] = 0
  END
[3899]   BLOCK (exit -> @3920)
[389D]     AWAIT gameflag_252A
[389E]     GUARD active_actor == Cyberquizz.talk (related 40)
[38A3]     ENDIF
[38A4]     SAY "Name a highly concentrated explosive word_65535 Big_bang splach plastok fuzz"  '[voice 10]
[38C2]     IF-BLOCK (exit -> @38EC)
[38C5]       GUARD concept == "splach"
[38C8]       ENDIF
[38C9]       SAY "Yes... Made from oxidized Larsen liver..."  '[voice 1, skip 3]
[38DD]       Bof += 1
[38E4]       quest += 1
[38EB]       CLEAR concept_alt
    END
[38EC]     IF-BLOCK (exit -> @3910)
[38EF]       GUARD NOT concept == "splach"
[38F3]       ENDIF
[38F4]       SAY "Well, that answer wasn't too explosive!"  '[voice 2, skip 2]
[3908]       quest += 1
[390F]       CLEAR concept_alt
    END
[3910]     SAY "Next question:"  '[voice 3, skip 1]
[391C]     POKE [0x389A] = 0
  END
[3920]   BLOCK (exit -> @39B9)
[3924]     AWAIT gameflag_252A
[3925]     GUARD active_actor == Cyberquizz.talk (related 40)
[392A]     ENDIF
[392B]     SAY "Who is the queen of the PAGO TAGO, wife of king Betakam IV? word_65535 umatika tina_burner scorpia pistilla etamina"  '[voice 3]
[395B]     IF-BLOCK (exit -> @398B)
[395E]       GUARD concept == "umatika"
[3961]       ENDIF
[3962]       SAY "Umatika , wife of king Betakam fourth, eighteenth dynasty..."  '[voice 4, skip 3]
[397C]       Bof += 1
[3983]       quest += 1
[398A]       CLEAR concept_alt
    END
[398B]     IF-BLOCK (exit -> @39A9)
[398E]       GUARD NOT concept == "umatika"
[3992]       ENDIF
[3993]       SAY "Not even close..."  '[voice 5, skip 2]
[39A1]       quest += 1
[39A8]       CLEAR concept_alt
    END
[39A9]     SAY "Next question:"  '[voice 3, skip 1]
[39B5]     POKE [0x3921] = 0
  END
[39B9]   BLOCK (exit -> @3A3E)
[39BD]     AWAIT gameflag_252A
[39BE]     GUARD active_actor == Cyberquizz.talk (related 40)
[39C3]     ENDIF
[39C4]     SAY "Who said Trump tails are dung? word_65535 lord_segelaxx inquisitor slim_gelati tromp_deustache"  '[voice 1]
[39E4]     IF-BLOCK (exit -> @3A08)
[39E7]       GUARD concept == "lord_segelaxx"
[39EA]       ENDIF
[39EB]       SAY "Avoid smoking them!"  '[voice 2, skip 3]
[39F9]       Bof += 1
[3A00]       quest += 1
[3A07]       CLEAR concept_alt
    END
[3A08]     IF-BLOCK (exit -> @3A2E)
[3A0B]       GUARD NOT concept == "lord_segelaxx"
[3A0F]       ENDIF
[3A10]       SAY "Well, maybe it was a tough question..."  '[voice 3, skip 2]
[3A26]       quest += 1
[3A2D]       CLEAR concept_alt
    END
[3A2E]     SAY "Next question:"  '[voice 3, skip 1]
[3A3A]     POKE [0x39BA] = 0
  END
[3A3E]   BLOCK (exit -> @3ACF)
[3A42]     AWAIT gameflag_252A
[3A43]     GUARD active_actor == Cyberquizz.talk (related 40)
[3A48]     ENDIF
[3A49]     SAY "On what planet is sex sold illicitly? word_65535 los_demonios venusia attroxcity trashtown"  '[voice 4]
[3A6B]     IF-BLOCK (exit -> @3A97)
[3A6E]       GUARD concept == "attroxcity"
[3A71]       ENDIF
[3A72]       SAY "Attroxcity, the Slimer city ... Quite right!"  '[voice 5, skip 3]
[3A88]       Bof += 1
[3A8F]       quest += 1
[3A96]       CLEAR concept_alt
    END
[3A97]     IF-BLOCK (exit -> @3ABF)
[3A9A]       GUARD NOT concept == "attroxcity"
[3A9E]       ENDIF
[3A9F]       SAY "No , that was a very wrong answer..."  '[voice 6, skip 2]
[3AB7]       quest += 1
[3ABE]       CLEAR concept_alt
    END
[3ABF]     SAY "Next question:"  '[voice 3, skip 1]
[3ACB]     POKE [0x3A3F] = 0
  END
[3ACF]   BLOCK (exit -> @3B68)
[3AD3]     AWAIT gameflag_252A
[3AD4]     GUARD active_actor == Cyberquizz.talk (related 40)
[3AD9]     ENDIF
[3ADA]     SAY "Who painted the famous portrait of Yolk? word_65535 van_ish van_gelis van_et van_deta van_tage van_gogue"  '[voice 7]
[3B00]     IF-BLOCK (exit -> @3B34)
[3B03]       GUARD concept == "van_gogue"
[3B06]       ENDIF
[3B07]       SAY "Van Gogue painted it with the Great One's own yellow blood..."  '[voice 8, skip 3]
[3B25]       Bof += 1
[3B2C]       quest += 1
[3B33]       CLEAR concept_alt
    END
[3B34]     IF-BLOCK (exit -> @3B58)
[3B37]       GUARD NOT concept == "van_gogue"
[3B3B]       ENDIF
[3B3C]       SAY "That was supposed to be easy..."  '[voice 9, skip 2]
[3B50]       quest += 1
[3B57]       CLEAR concept_alt
    END
[3B58]     SAY "Next question:"  '[voice 3, skip 1]
[3B64]     POKE [0x3AD0] = 0
  END
[3B68]   BLOCK (exit -> @3BEB)
[3B6C]     AWAIT gameflag_252A
[3B6D]     GUARD active_actor == Cyberquizz.talk (related 40)
[3B72]     ENDIF
[3B73]     SAY "Who wrote the famous Croolicum? word_65535 lord_raptor lord_ship von_spacecraft nobody"  '[voice 10]
[3B91]     IF-BLOCK (exit -> @3BBD)
[3B94]       GUARD concept == "nobody"
[3B97]       ENDIF
[3B98]       SAY "There's no catching you out , eh?"  '[voice 0, skip 3]
[3BAE]       Bof += 1
[3BB5]       quest += 1
[3BBC]       CLEAR concept_alt
    END
[3BBD]     IF-BLOCK (exit -> @3BDB)
[3BC0]       GUARD NOT concept == "nobody"
[3BC4]       ENDIF
[3BC5]       SAY "What's the Croolicum?"  '[voice 1, skip 2]
[3BD3]       quest += 1
[3BDA]       CLEAR concept_alt
    END
[3BDB]     SAY "Next question:"  '[voice 3, skip 1]
[3BE7]     POKE [0x3B69] = 0
  END
[3BEB]   BLOCK (exit -> @3C6C)
[3BEF]     AWAIT gameflag_252A
[3BF0]     GUARD active_actor == Cyberquizz.talk (related 40)
[3BF5]     ENDIF
[3BF6]     SAY "Who was the first Trump tail smoker in the universe? word_65535 al_hure sebasto_paul lord_sirtaki jeph_dulikan"  '[voice 2]
[3C1E]     IF-BLOCK (exit -> @3C42)
[3C21]       GUARD concept == "al_hure"
[3C24]       ENDIF
[3C25]       SAY "Good ol' Al..."  '[voice 2, skip 3]
[3C33]       Bof += 1
[3C3A]       quest += 1
[3C41]       CLEAR concept_alt
    END
[3C42]     IF-BLOCK (exit -> @3C5C)
[3C45]       GUARD NOT concept == "al_hure"
[3C49]       ENDIF
[3C4A]       SAY "Sorry..."  '[voice 3, skip 2]
[3C54]       quest += 1
[3C5B]       CLEAR concept_alt
    END
[3C5C]     SAY "Next question:"  '[voice 3, skip 1]
[3C68]     POKE [0x3BEC] = 0
  END
[3C6C]   BLOCK (exit -> @3CF5)
[3C70]     AWAIT gameflag_252A
[3C71]     GUARD active_actor == Cyberquizz.talk (related 40)
[3C76]     ENDIF
[3C77]     SAY "Who said :"Silence feeds on thunder and fury"? word_65535 yolk joy_stica hom super_zen"  '[voice 4]
[3C9B]     IF-BLOCK (exit -> @3CC5)
[3C9E]       GUARD concept == "yolk"
[3CA1]       ENDIF
[3CA2]       SAY "Peace to his yellow soul... Bravo."  '[voice 4, skip 3]
[3CB6]       Bof += 1
[3CBD]       quest += 1
[3CC4]       CLEAR concept_alt
    END
[3CC5]     IF-BLOCK (exit -> @3CE5)
[3CC8]       GUARD NOT concept == "yolk"
[3CCC]       ENDIF
[3CCD]       SAY "Read your classics ..."  '[voice 5, skip 2]
[3CDD]       quest += 1
[3CE4]       CLEAR concept_alt
    END
[3CE5]     SAY "Next question:"  '[voice 3, skip 1]
[3CF1]     POKE [0x3C6D] = 0
  END
[3CF5]   BLOCK (exit -> @3D70)
[3CF9]     AWAIT gameflag_252A
[3CFA]     GUARD active_actor == Cyberquizz.talk (related 40)
[3CFF]     ENDIF
[3D00]     SAY "Name one hit by the Migrators word_65535 crush_me_baby space_maker hello_dolly love_me_do"  '[voice 6]
[3D20]     IF-BLOCK (exit -> @3D44)
[3D23]       GUARD concept == "crush_me_baby"
[3D26]       ENDIF
[3D27]       SAY "Cruuuush meee baaaabbbyyyy!"  '[voice 7, skip 3]
[3D35]       Bof += 1
[3D3C]       quest += 1
[3D43]       CLEAR concept_alt
    END
[3D44]     IF-BLOCK (exit -> @3D60)
[3D47]       GUARD NOT concept == "crush_me_baby.."
[3D4B]       ENDIF
[3D4C]       SAY "Ouch ..."  '[voice 8, skip 2]
[3D58]       quest += 1
[3D5F]       CLEAR concept_alt
    END
[3D60]     SAY "Next question:"  '[voice 3, skip 1]
[3D6C]     POKE [0x3CF6] = 0
  END
[3D70]   BLOCK (exit -> @3DF5)
[3D74]     AWAIT gameflag_252A
[3D75]     GUARD active_actor == Cyberquizz.talk (related 40)
[3D7A]     ENDIF
[3D7B]     SAY "Who is the oldest living being in the universe? word_65535 otto_von_smile Bob_Morlock super_tromp techno_paul"  '[voice 8]
[3DA1]     IF-BLOCK (exit -> @3DC5)
[3DA4]       GUARD concept == "Bob_Morlock"
[3DA7]       ENDIF
[3DA8]       SAY "Good ol' Bob!"  '[voice 8, skip 3]
[3DB6]       Bof += 1
[3DBD]       quest += 1
[3DC4]       CLEAR concept_alt
    END
[3DC5]     IF-BLOCK (exit -> @3DE5)
[3DC8]       GUARD NOT concept == "Bob_Morlock"
[3DCC]       ENDIF
[3DCD]       SAY "I said the OLDEST..."  '[voice 6, skip 2]
[3DDD]       quest += 1
[3DE4]       CLEAR concept_alt
    END
[3DE5]     SAY "Next question:"  '[voice 3, skip 1]
[3DF1]     POKE [0x3D71] = 0
  END
[3DF5]   BLOCK (exit -> @3E84)
[3DF9]     AWAIT gameflag_252A
[3DFA]     GUARD active_actor == Cyberquizz.talk (related 40)
[3DFF]     ENDIF
[3E00]     SAY "Which famous dancer starred in "Squeeze me in your tentacles darling"? word_65535 torka tina_burner cybertha joy_stika"  '[voice 8]
[3E2A]     IF-BLOCK (exit -> @3E50)
[3E2D]       GUARD concept == "tina_burner"
[3E30]       ENDIF
[3E31]       SAY "What a performance !"  '[voice 6, skip 3]
[3E41]       Bof += 1
[3E48]       quest += 1
[3E4F]       CLEAR concept_alt
    END
[3E50]     IF-BLOCK (exit -> @3E74)
[3E53]       GUARD NOT concept == "tina_burner"
[3E57]       ENDIF
[3E58]       SAY "How can you not know that?"  '[voice 9, skip 2]
[3E6C]       quest += 1
[3E73]       CLEAR concept_alt
    END
[3E74]     SAY "Next question:"  '[voice 3, skip 1]
[3E80]     POKE [0x3DF6] = 0
  END
[3E84]   BLOCK (exit -> @3F0D)
[3E88]     AWAIT gameflag_252A
[3E89]     GUARD active_actor == Cyberquizz.talk (related 40)
[3E8E]     ENDIF
[3E8F]     SAY "What is the preferred drink of the Migrax? word_65535 processed_liver frozen_migrita fermented_egg heavy_water"  '[voice 6]
[3EB3]     IF-BLOCK (exit -> @3ED7)
[3EB6]       GUARD concept == "processed_liver"
[3EB9]       ENDIF
[3EBA]       SAY "Way too easy..."  '[voice 9, skip 3]
[3EC8]       Bof += 1
[3ECF]       quest += 1
[3ED6]       CLEAR concept_alt
    END
[3ED7]     IF-BLOCK (exit -> @3EFD)
[3EDA]       GUARD NOT concept == "processed_liver"
[3EDE]       ENDIF
[3EDF]       SAY "You mean you don't know the Migrax?"  '[voice 3, skip 2]
[3EF5]       quest += 1
[3EFC]       CLEAR concept_alt
    END
[3EFD]     SAY "Next question:"  '[voice 3, skip 1]
[3F09]     POKE [0x3E85] = 0
  END
[3F0D]   BLOCK (exit -> @3F9E)
[3F11]     AWAIT gameflag_252A
[3F12]     GUARD active_actor == Cyberquizz.talk (related 40)
[3F17]     ENDIF
[3F18]     SAY "Who said:"ondoyant pretty, Croolis drool. ondoyant ugly, Croolis cruel" word_65535 nobody maxxon yolk"  '[voice 6]
[3F3C]     IF-BLOCK (exit -> @3F66)
[3F3F]       GUARD concept == "maxxon"
[3F42]       ENDIF
[3F43]       SAY "What a poet , that Maxxon."  '[voice 0, skip 3]
[3F57]       Bof += 1
[3F5E]       quest += 1
[3F65]       CLEAR concept_alt
    END
[3F66]     IF-BLOCK (exit -> @3F8E)
[3F69]       GUARD NOT concept == "maxxon"
[3F6D]       ENDIF
[3F6E]       SAY "No poetry in your withered soul , eh?"  '[voice 3, skip 2]
[3F86]       quest += 1
[3F8D]       CLEAR concept_alt
    END
[3F8E]     SAY "Next question:"  '[voice 3, skip 1]
[3F9A]     POKE [0x3F0E] = 0
  END
[3F9E]   BLOCK (exit -> @402D)
[3FA2]     AWAIT gameflag_252A
[3FA3]     GUARD active_actor == Cyberquizz.talk (related 40)
[3FA8]     ENDIF
[3FA9]     SAY "The Great Yolk's tomb is on which planet? word_65535 rondo ekatomb vista magnus"  '[voice 0]
[3FCD]     IF-BLOCK (exit -> @3FF7)
[3FD0]       GUARD concept == "vista"
[3FD3]       ENDIF
[3FD4]       SAY "In the SCRUT palace on Vista"  '[voice 6, skip 3]
[3FE8]       Bof += 1
[3FEF]       quest += 1
[3FF6]       CLEAR concept_alt
    END
[3FF7]     IF-BLOCK (exit -> @401D)
[3FFA]       GUARD NOT concept == "vista"
[3FFE]       ENDIF
[3FFF]       SAY "You never heard of the SCRUT palace?"  '[voice 9, skip 2]
[4015]       quest += 1
[401C]       CLEAR concept_alt
    END
[401D]     SAY "Next question:"  '[voice 3, skip 1]
[4029]     POKE [0x3F9F] = 0
  END
[402D]   BLOCK (exit -> @40B4)
[4031]     AWAIT gameflag_252A
[4032]     GUARD active_actor == Cyberquizz.talk (related 40)
[4037]     ENDIF
[4038]     SAY "On which planet may the voice be heard? word_65535 pterra cyberock kult tumul"  '[voice 7]
[405C]     IF-BLOCK (exit -> @4080)
[405F]       GUARD concept == "kult"
[4062]       ENDIF
[4063]       SAY "Indeed , Commander..."  '[voice 3, skip 3]
[4071]       Bof += 1
[4078]       quest += 1
[407F]       CLEAR concept_alt
    END
[4080]     IF-BLOCK (exit -> @40A4)
[4083]       GUARD NOT concept == "kult"
[4087]       ENDIF
[4088]       SAY "You're not deaf by any chance?"  '[voice 0, skip 2]
[409C]       quest += 1
[40A3]       CLEAR concept_alt
    END
[40A4]     SAY "next question:"  '[voice 3, skip 1]
[40B0]     POKE [0x402E] = 0
  END
[40B4]   BLOCK (exit -> @413F)
[40B8]     AWAIT gameflag_252A
[40B9]     GUARD active_actor == Cyberquizz.talk (related 40)
[40BE]     ENDIF
[40BF]     SAY "What is the planet Tumul's sun? word_65535 ex897 gladis corpo negratom"  '[voice 1]
[40DF]     IF-BLOCK (exit -> @4109)
[40E2]       GUARD concept == "gladis"
[40E5]       ENDIF
[40E6]       SAY "Gladis , the smart star ."  '[voice 3, skip 3]
[40FA]       Bof += 1
[4101]       quest += 1
[4108]       CLEAR concept_alt
    END
[4109]     IF-BLOCK (exit -> @412F)
[410C]       GUARD NOT concept == "gladis"
[4110]       ENDIF
[4111]       SAY "You mean you never went to Tumul?"  '[voice 1, skip 2]
[4127]       quest += 1
[412E]       CLEAR concept_alt
    END
[412F]     SAY "Next question:"  '[voice 3, skip 1]
[413B]     POKE [0x40B5] = 0
  END
[413F]   BLOCK (exit -> @41CE)
[4143]     AWAIT gameflag_252A
[4144]     GUARD active_actor == Cyberquizz.talk (related 40)
[4149]     ENDIF
[414A]     SAY "How many moons has Moskito? word_65535 five twelve zero thirty_two"  '[voice 10]
[4168]     IF-BLOCK (exit -> @4190)
[416B]       GUARD concept == "zero"
[416E]       ENDIF
[416F]       SAY "Absolutely right. Moskito is moonless."  '[voice 1, skip 3]
[4181]       Bof += 1
[4188]       quest += 1
[418F]       CLEAR concept_alt
    END
[4190]     IF-BLOCK (exit -> @41B6)
[4193]       GUARD NOT concept == "zero"
[4197]       ENDIF
[4198]       SAY "Maybe you have mosquitoes for a brain"  '[voice 3, skip 2]
[41AE]       quest += 1
[41B5]       CLEAR concept_alt
    END
[41B6]     SAY "And that wraps it up question-wise..."  '[voice 3, skip 1]
[41CA]     POKE [0x4140] = 0
  END
[41CE]   BLOCK (exit -> @4510)
[41D2]     AWAIT gameflag_274F
[41D3]     GUARD active_actor == Scruter_Jo.talk (related 40)
[41D8]     ENDIF
[41D9]     SAY "Commander , you go search BIONIUM in SCRUTER JO's cyberspace ..."  '[voice 19]
[41F7]     IF-BLOCK (exit -> @43A6)
[41FA]       GUARD compris == 0
[4201]       ENDIF
[4202]       SAY "Me explain how you pick up BIONIUM .."
[421A]       SAY "You find BIOXX . Bioxx be small energy creatures ..."
[4236]       SAY "You touch BIOXX with hand once ."  '[voice 1]
[424C]       SAY "Sounds like a piece of cake to me , Commander !!"
[426A]       SAY "If you touch BIOXX twice , you CAPTURE BIOXX on tip of your finger ..."  '[voice 2]
[4290]       SAY "You catch it on the tip of your finger !!! How about that ..."
[42B4]       SAY "Then you can carry BIOXX to Cybernetic MANTAS"  '[voice 3]
[42CC]       SAY "You put BIOXX in belly of Manta ..."  '[voice 5]
[42E4]       SAY "BIOXX stick to MANTAS ..."  '[voice 4]
[42F6]       SAY "That's something I wanna see ..."
[430A]       SAY "Mantas change BIOXX into BIONIUM..."  '[voice 6]
[431C]       SAY "More BIOXX you give to Mantas , more BIONIUM you get ..."  '[voice 5]
[433C]       SAY "Lead me to those BIOXXes !!! BIONIUM ... Yummy ..."
[4358]       SAY "To come back from CYBERSPACE , you touch BLUE BOX ...."  '[voice 4]
[4376]       SAY "You understand ?"  '[voice 6]
[4384]       SAY "We get the picture , Mister SCRUTER JO ... Right , Commander ?"
    END
[43A6]     SAY "YOU go , Commander ..."  '[voice 20]
[43B8]     SAY "Ahh ... Me feel better ..."  '[voice 21]
[43CC]     IF-BLOCK (exit -> @4456)
[43CF]       GUARD vbio > 0
[43D6]       ENDIF
[43D7]       SAY "Good work ... You did be successful ..."  '[voice 2]
[43EF]       SAY "You did get BIONIUM ..."  '[voice 3]
[4401]       SAY "YES !!! Commander , did I ever tell you what a cool dude you are ?"
[4429]       SAY "The BIONIUM will make me smarter and I'll be able to help you better ..."
[444F]       compris = 1
    END
[4456]     IF-BLOCK (exit -> @44F1)
[4459]       GUARD vbio == 0
[4460]       ENDIF
[4461]       SAY "Not good , friend ... You fail ..."  '[voice 4]
[4479]       SAY "You didn't understand the idea , huh , Commander ..."
[4495]       SAY "I have to have BIONIUM , Commander . It makes me smarter ..."
[44B7]       SAY "Ha ! Ha ! You need much bionium BIONIUM ... Ha ! Ha ! ..."  '[voice 4]
[44DD]       SAY "That's enough out of you !!!"
    END
[44F1]     SAY "Bye bye , Commander . Me go back to CYBERSPACE..."  '[voice 7, skip 1]
[450D]     END PRESENTATION Scruter_Jo.talk
  END
[4510]   BLOCK (exit -> @46A3)
[4514]     AWAIT gameflag_252A
[4515]     GUARD rec_07B2 == 3884
[451A]     GUARD active_actor == Migrator.talk (related 40)
[451F]     ENDIF
[4520]     SAY "Me greet you , friend ..."  '[voice 5]
[4534]     SAY "You have ring ???..."  '[voice 2]
[4544]     SAY "Me soon marry TINA... YOU COME WEDDING . ME INVITE YOU . WE MAKE BIG CONCERT ..."  '[voice 3]
[456E]     IF-BLOCK (exit -> @4650)
[4571]       GUARD rec_12F0 == 40
[4576]       ENDIF
[4577]       SAY "Teleport him the ring , Commander . That'll make his day ...."
[4597]       SAY "RING TELEPORTED TO MIGRATOR word_65535 TELEPORT"
[45AD]       IF-BLOCK (exit -> @45D2)
[45B0]         GUARD concept == "TELEPORT"
[45B3]         ENDIF
[45B4]         SAY "RING TELEPORTED TO MOSKITO AIRPORT ZONE ."  '[skip 2]
[45CA]         OP_CD CD 30 00 DC 12 3A 04
[45D1]         CLEAR concept_alt
      END
[45D2]       SAY "WOWWWW ! Ring be nice nice ... Me like ... Me thank you , friend ...."  '[voice 2]
[45FA]       SAY "Thank you ... Thank you ... YOU COME WEDDING ... WE MAKE BIG CONCERT ..."  '[voice 3]
[4620]       SAY "BYE BYE FRIEND .... TINA SEND YOU BIG KISS ..."  '[voice 2]
[463C]       SAY "..."  '[skip 2]
[4646]       mariage = 1
[464D]       END PRESENTATION Migrator.talk
    END
[4650]     SAY "You not have ring ... Me worried friend ..."  '[voice 2]
[466A]     SAY "You quick go get ring ..."  '[voice 3]
[467E]     SAY "Me wait you . Bye bye friend ..."  '[voice 2]
[4696]     SAY "..."  '[skip 1]
[46A0]     END PRESENTATION Migrator.talk
  END
[46A3]   BLOCK (exit -> @46BB)
[46A7]     AWAIT presentation
[46A8]     GUARD rec_00AA == 4024
[46AD]     ENDIF
[46AE]     OP_C3 C3 44 07 28 00
[46B3]     POKE [0x46BC] = 1
[46B7]     POKE [0x46A4] = 0
  END
[46BB]   GOTO @480C
[46BF]   AWAIT presentation
[46C0]   ENDIF
[46C1]   SAY "Come in come in ...THIS BE HANNA SCRUTA . My husband SCRUTER JO be fighter pilot ..."
[46EB]   SAY "You blow up his fighter in combat ... ME WIDOW NOW ... CRY ... CRY ..."
[4713]   SAY "ME NOT CAN PAY RENT ... YOUR FAULT ... INSURANCE NOT GIVE ANYTHING ..."
[4737]   SAY "ME WIDOW ... CRY ... CRY ... DESPAIRING WAIL .... YOU KILL SCRUTER JO ..."
[475D]   SAY "Commander ? They're giving out your phone number to widows now ..."
[477D]   SAY "It's not right . They're trying to psych you out , Commander ... Don't answer ..."
[47A5]   SAY "YOU MUST PAY ... CRY ... CRY ... YOU DID KILL MY SCRUTER JO ... CRY ... CRY ...."
[47D3]   SAY "KRUIKKK..."
[47DD]   SAY "She hung up ... I don't like it , Commander ..."
[47FB]   SAY "stop"  '[skip 2]
[4805]   POKE [0x46BC] = 0
[4809]   END PRESENTATION Scruter_K.talk
[480C]   BLOCK (exit -> @48BF)
[4810]     AWAIT presentation
[4811]     GUARD active_actor == menu.talk (related 40)
[4816]     GUARD rec_037A == 65535
[481B]     ENDIF
[481C]     SAY ""IMPROVED MENU""
[4828]     SAY "Today CHEF BRONKO has laid out for you :"
[4842]     SAY "Tasty MUFFALO soup Bronko-style ."
[4854]     SAY "MURFFALO kidneys Bronko-style ."
[4864]     SAY "MURFFALO hamburger with Bar-B-Q recycled-oil dip ."
[487A]     SAY "Smooth MURFFALO-chip ice cream ."
[488C]     SAY "Recycled water"
[4898]     SAY "Chef Bronko says ... Burping's bad manners ! ..."
[48B2]     SAY "stop"  '[skip 1]
[48BC]     END PRESENTATION menu.talk
  END
[48BF]   BLOCK (exit -> @496F)
[48C3]     AWAIT presentation
[48C4]     GUARD active_actor == menu.talk (related 40)
[48C9]     GUARD NOT rec_037A == 65535
[48CF]     ENDIF
[48D0]     SAY ""MENU""
[48DA]     SAY "Today's fare :"
[48E8]     SAY "PLASMA soup HONK-style ."
[48F8]     SAY "WRIGGLER belly in slobber sauce ."
[490C]     SAY "Jellied URTIKAN with MURFFALO bone marrow ."
[4922]     SAY "GLOK eye pie ."
[4932]     SAY "Recycled water"
[493E]     SAY "The chef says ... Don't eat with your mouth full ! ..."
[495E]     SAY "Stop"  '[skip 2]
[4968]     POKE [0x48C0] = 0
[496C]     END PRESENTATION menu.talk
  END
[496F]   BLOCK (exit -> @4A19)
[4973]     AWAIT presentation
[4974]     GUARD active_actor == menu.talk (related 40)
[4979]     GUARD NOT rec_037A == 65535
[497F]     ENDIF
[4980]     SAY ""MENU""
[498A]     SAY "Today's fare :"
[4998]     SAY "PLASMA soup HONK-style ."
[49A8]     SAY "WRIGGLER snout stew ."
[49B8]     SAY "URTIKAN seeds in MURFFALO venom ."
[49CC]     SAY "GLOK juice dessert ."
[49DC]     SAY "Recycled water"
[49E8]     SAY "The chef says ... Don't talk with your mouth open ! ..."
[4A08]     SAY "stop"  '[skip 2]
[4A12]     POKE [0x4970] = 0
[4A16]     END PRESENTATION menu.talk
  END
[4A19]   BLOCK (exit -> @4AC5)
[4A1D]     AWAIT presentation
[4A1E]     GUARD NOT rec_037A == 65535
[4A24]     GUARD active_actor == menu.talk (related 40)
[4A29]     ENDIF
[4A2A]     SAY ""MENU""
[4A34]     SAY "Today's fare :"
[4A42]     SAY "PLASMA soup HONK-style ."
[4A52]     SAY "WRIGGLER feet in emulsive sauce ."
[4A66]     SAY "URTIKAN leaves in MURFFALO sweat ."
[4A7A]     SAY "GLOK flake dessert ."
[4A8A]     SAY "Recycled water"
[4A96]     SAY "The chef says ... Somebody didn't finish his wrigglers yesterday ..."
[4AB4]     SAY "stop"  '[skip 2]
[4ABE]     POKE [0x4A1A] = 0
[4AC2]     END PRESENTATION menu.talk
  END
[4AC5]   BLOCK (exit -> @4B79)
[4AC9]     AWAIT presentation
[4ACA]     GUARD active_actor == menu.talk (related 40)
[4ACF]     GUARD NOT rec_037A == 65535
[4AD5]     ENDIF
[4AD6]     SAY ""MENU""
[4AE0]     SAY "Today's fare :"
[4AEE]     SAY "HONK-style PLASMA soup ."
[4AFE]     SAY "WRIGGLER brain , stewed in its own juice ."
[4B18]     SAY "URTIKAN trunk , stuffed with MURFFALO liver ."
[4B30]     SAY "GLOK dee-lite ."
[4B3E]     SAY "Recycled water"
[4B4A]     SAY "The chef says ... Plenty more in the kitchen ! ..."
[4B68]     SAY "stop"  '[skip 2]
[4B72]     POKE [0x4AC6] = 0
[4B76]     END PRESENTATION menu.talk
  END
[4B79]   BLOCK (exit -> @4C35)
[4B7D]     AWAIT presentation
[4B7E]     GUARD active_actor == menu.talk (related 40)
[4B83]     GUARD NOT rec_037A == 65535
[4B89]     ENDIF
[4B8A]     SAY ""IMPROVED MENU""
[4B96]     SAY "Today's fare :"
[4BA4]     SAY "Soup of PLASMA HONK-style ."
[4BB6]     SAY "WRIGGLER hearts in green blood coagulate ."
[4BCC]     SAY "URTIKAN roots , deep fried in recycled oil ."
[4BE6]     SAY "Candied GLOK tongue ."
[4BF6]     SAY "Recycled water"
[4C02]     SAY "The chef says ... You eat what you are ! ..."
[4C20]     SAY "stop"  '[skip 3]
[4C2A]     POKE [0x4B7A] = 0
[4C2E]     POKE [0x4C36] = 1
[4C32]     END PRESENTATION menu.talk
  END
[4C35]   GOTO @4C52
[4C39]   ENDIF
[4C3A]   POKE [0x48C0] = 1
[4C3E]   POKE [0x4970] = 1
[4C42]   POKE [0x4A1A] = 1
[4C46]   POKE [0x4AC6] = 1
[4C4A]   POKE [0x4B7A] = 1
[4C4E]   POKE [0x4C36] = 0
[4C52]   BLOCK (exit -> @4E17)
[4C56]     AWAIT gameflag_274F
[4C57]     GUARD active_actor == Bob_Morlock.talk (related 40)
[4C5C]     GUARD revelat == 0
[4C63]     ENDIF
[4C64]     SAY "Would you care to know an unbearable truth , Commander ?"  '[voice 7]
[4C82]     SAY "HONK ! Switch yourself off for ten seconds !!!"  '[voice 6]
[4C9C]     SAY "But , Cap'n Bob ! I ..."
[4CB2]     SAY "SWITCH OFF I SAID !!!"  '[voice 5]
[4CC4]     SAY "Yes sir ....."
[4CD2]     SAY "KRUIIIIK !!! AAAaaaaaaaaaaaaahhhhh !!!"
[4CE2]     SAY "COMMANDER , YOU ARE ME ...."  '[voice 5]
[4CF6]     SAY "WE ARE THE SAME BEING AT TWO DIFFERENT AGES ..."  '[voice 6]
[4D12]     SAY "YOU ARE MORE THAN MY SON ..."  '[voice 4]
[4D28]     SAY "We are the same person ... I am the first being to create itself ..."  '[voice 5]
[4D4E]     SAY "And , thanks to space-time contorsion, to watch itself relive : YOU ARE BOB , COMMANDER ..."  '[voice 4]
[4D78]     SAY "I am what you'll be in a few hundred thousand years ..."  '[voice 2]
[4D98]     SAY "OK Honk , you can switch yourself back on ..."  '[voice 6]
[4DB4]     SAY "KROIIIIkkk !!! -&KRUIIIIkkk !!! -&I AM supposed to be aware of everything that goes on round here , being the onboard computer ... Don't tell me you switched off Olga ???"
[4DFA]     SAY "Can it and do some work !!"  '[voice 5, skip 2]
[4E10]     POKE [0x4C53] = 0
[4E14]     END PRESENTATION Bob_Morlock.talk
  END
[4E17]   BLOCK (exit -> @51E2)
[4E1B]     AWAIT presentation
[4E1C]     GUARD active_actor == Honk.talk (related 40)
[4E21]     ENDIF
[4E22]     SAY "I exist only to obey , Commander"
[4E38]     IF-BLOCK (exit -> @4EAD)
[4E3B]       GUARD vbio == 0
[4E42]       ENDIF
[4E43]       SAY "Commander , we don't have any BIONIUM ... COMMANDER , please ..."
[4E63]       SAY "I need that energy ..."
[4E75]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
[4E8B]       SAY "Wake up Scruter Jo , Commander . He's sleeping in the Cryobox ..."
    END
[4EAD]     IF-BLOCK (exit -> @4F0E)
[4EB0]       GUARD vbio == 1
[4EB7]       ENDIF
[4EB8]       SAY "We've got one dose of BIONIUM left , Commander"
[4ED2]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
[4EE8]       SAY "I don't feel too sure of myself , Commander... I really need that energy ..."
    END
[4F0E]     IF-BLOCK (exit -> @4F49)
[4F11]       GUARD vbio == 2
[4F18]       ENDIF
[4F19]       SAY "We've got two doses of BIONIUM left , Commander"
[4F33]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
    END
[4F49]     IF-BLOCK (exit -> @4F6E)
[4F4C]       GUARD vbio == 3
[4F53]       ENDIF
[4F54]       SAY "We've got three doses of BIONIUM left , Commander"
    END
[4F6E]     IF-BLOCK (exit -> @4F93)
[4F71]       GUARD vbio == 4
[4F78]       ENDIF
[4F79]       SAY "We've got four doses of BIONIUM left , Commander"
    END
[4F93]     IF-BLOCK (exit -> @4FB8)
[4F96]       GUARD vbio == 5
[4F9D]       ENDIF
[4F9E]       SAY "We've got five doses of BIONIUM left , Commander"
    END
[4FB8]     IF-BLOCK (exit -> @4FDD)
[4FBB]       GUARD vbio == 6
[4FC2]       ENDIF
[4FC3]       SAY "We've got six doses of BIONIUM left , Commander"
    END
[4FDD]     IF-BLOCK (exit -> @5002)
[4FE0]       GUARD vbio == 7
[4FE7]       ENDIF
[4FE8]       SAY "We've got seven doses of BIONIUM left , Commander"
    END
[5002]     IF-BLOCK (exit -> @503B)
[5005]       GUARD vbio == 8
[500C]       ENDIF
[500D]       SAY "We've got eight doses of BIONIUM left , Commander"
[5027]       SAY "You're the best , Commander ..."
    END
[503B]     IF-BLOCK (exit -> @5088)
[503E]       GUARD vbio > 2
[5045]       GUARD rec_0230 < 2
[504C]       GUARD (airport.talk & 0x2) == 0
[5052]       ENDIF
[5053]       SAY "Commander , I'm sure that HOM is a vital link in the chain that leads to the Big Bang..."  '[skip 1]
[5081]       vbio -= 3
    END
[5088]     IF-BLOCK (exit -> @50C6)
[508B]       GUARD vbio > 2
[5092]       GUARD bok == 0
[5099]       GUARD rec_0470 > 0
[50A0]       ENDIF
[50A1]       SAY "Commander , Mister Bob want to see you in the Cryobox..."  '[skip 1]
[50BF]       vbio -= 3
    END
[50C6]     IF-BLOCK (exit -> @50FC)
[50C9]       GUARD vbio > 2
[50D0]       GUARD rec_025A == 65535
[50D5]       GUARD rec_0278 == 0
[50DC]       ENDIF
[50DD]       SAY "Commander, don't forget Yoko and Maxxon asleep CRYOBOX..."  '[skip 1]
[50F5]       vbio -= 3
    END
[50FC]     IF-BLOCK (exit -> @5146)
[50FF]       GUARD vbio > 2
[5106]       GUARD rec_025A == 65535
[510B]       GUARD rec_0278 > 0
[5112]       ENDIF
[5113]       SAY "Approach the planet RONDO and talk to Yoko or Maxxon in the CRYOBOX in order to teleport them..."  '[skip 1]
[513F]       vbio -= 3
    END
[5146]     IF-BLOCK (exit -> @5175)
[5149]       GUARD vbio > 2
[5150]       GUARD rec_00C8 == 0
[5157]       ENDIF
[5158]       SAY "Better go see Bug Deluxe on Venusia..."  '[skip 1]
[516E]       vbio -= 3
    END
[5175]     IF-BLOCK (exit -> @51B6)
[5178]       GUARD vbio > 2
[517F]       GUARD rec_12F0 == 938
[5184]       GUARD Bigband == est.value
[518B]       GUARD rec_0470 > 0
[5192]       ENDIF
[5193]       SAY "Commander... Maybe the ondoyant has a ring for us ..."  '[skip 1]
[51AF]       vbio -= 3
    END
[51B6]     SAY "More information ?"  '[skip 1]
[51C4]     rec_0900 = 1
[51C9]     SAY "Bye bye , Commander ... word_65535 bye_bye"  '[skip 1]
[51DF]     END PRESENTATION Honk.talk
  END
[51E2] END OF SCRIPT
