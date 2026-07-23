[0000]   BLOCK (exit -> @00C8)
[0004]     ENDIF
[0005]     rec_1006 |= 0x2
[000A]     rec_0E1A |= 0x2
[000F]     rec_11CA = 65535
[0014]     rec_0A24 |= 0x2
[0019]     rec_0D06 |= 0x2
[001E]     rec_0C2E |= 0x2
[0023]     rec_0FCA |= 0x2
[0028]     rec_0A7E |= 0x2
[002D]     rec_0BF2 |= 0x2
[0032]     rec_0AEA |= 0x2
[0037]     rec_01B4 |= 0x2
[003C]     rec_094C |= 0x1
[0041]     rec_0514 |= 0x1
[0046]     rec_0962 = 4130
[004B]     rec_0182 = 65535
[0050]     rec_0452 = 65535
[0055]     rec_03C2 = 4246
[005A]     rec_0332 = 4070
[005F]     rec_040A = 4070
[0064]     rec_064A = 4070
[0069]     rec_0692 = 4070
[006E]     rec_02EA = 3056
[0073]     rec_0722 = 4070
[0078]     compris = 1
[007F]     vbio = 3
[0086]     SETCHAR slot 1 = "microkid"
[0092]     SETCHAR slot 2 = "ppit"
[009A]     SETCHAR slot 3 = "hatetv"
[00A4]     SETCHAR slot 4 = "venus"
[00AD]     SETCHAR slot 5 = "scrut"
[00B6]     SETCHAR slot 6 = "match"
[00BF]     rec_04E2 = 3332
[00C4]     POKE [0x0001] = 0
  END
[00C8]   BLOCK (exit -> @0406)
[00CC]     AWAIT gameflag_274F
[00CD]     GUARD active_actor == Scruter_Jo.talk (related 40)
[00D2]     ENDIF
[00D3]     SAY "Commander you go get BIONIUM in CYBERSPACE of SCRUTER JO..."  '[voice 19]
[00EF]     IF-BLOCK (exit -> @0296)
[00F2]       GUARD compris == 0
[00F9]       ENDIF
[00FA]       SAY "Me explain to you how get BIONIUM..."
[0110]       SAY "You find BIOXX . Bioxx be small energy creatures ..."
[012C]       SAY "You touch BIOXX once with hand ."  '[voice 1]
[0142]       SAY "Sounds like a piece of cake to me , Commander !!"
[0160]       SAY "If you touch BIOXX twice , you CAPTURE BIOXX on tip of your finger ..."  '[voice 2]
[0186]       SAY "Catch him on the tip of your finger !!! Sounds easy as pie , Commander ..."
[01AE]       SAY "Then you can carry BIOXX to cybernetic MANTAS"  '[voice 3]
[01C6]       SAY "You place BIOXX in belly of Manta..."  '[voice 5]
[01DC]       SAY "BIOXX stay stuck to MANTAS..."  '[voice 4]
[01EE]       SAY "I'd love to see that ..."
[0202]       SAY "Mantas change BIOXX into BIONIUM..."  '[voice 6]
[0214]       SAY "More BIOXX you give to Mantas, more BIONIUM you get ..."  '[voice 5]
[0232]       SAY "Yes !!! BIONIUM ... I can taste it already ..."
[024E]       SAY "To come back from CYBERSPACE , you touch BLUE BOX ...."  '[voice 4]
[026C]       SAY "You understand ?"  '[voice 6]
[027A]       SAY "We understand perfectly , Mister SCRUTER JO... Right , Commander?"
    END
[0296]     SAY "YOU go , Commander ..."  '[voice 20]
[02A8]     SAY "Ahh! Me feel better ..."  '[voice 21]
[02BA]     IF-BLOCK (exit -> @0360)
[02BD]       GUARD vbio > 0
[02C4]       ENDIF
[02C5]       SAY "Good work ... You did succeed ..."  '[voice 2]
[02DB]       SAY "You did get BIONIUM..."  '[voice 3]
[02EB]       SAY "YES !!! Commander , remind me to tell you you're a champ ..."
[030D]       SAY "This BIONIUM is extraordinary . My clock frequency's through the roof ..."
[032D]       SAY "I feel even smarter ... I can feel I'll be a great help to you , Commander ..."  '[skip 1]
[0359]       compris = 1
    END
[0360]     IF-BLOCK (exit -> @03E9)
[0363]       GUARD vbio == 0
[036A]       ENDIF
[036B]       SAY "Not good , friend ... You fail ..."  '[voice 4]
[0383]       SAY "Commander, you didn't understand the technique ..."
[0399]       SAY "I need BIONIUM, Commander . It makes me smarter ..."
[03B5]       SAY "Ha! Ha! You need much much BIONIUM . Ha! Ha!..."  '[voice 4]
[03D1]       SAY "Why don't you shut up , wiseguy !!!"
    END
[03E9]     SAY "Bye bye , Commander . Me return to CYBERSPACE..."  '[voice 7, skip 1]
[0403]     END PRESENTATION Scruter_Jo.talk
  END
[0406]   BLOCK (exit -> @04A4)
[040A]     AWAIT gameflag_252A
[040B]     GUARD rec_1088 == 3332
[0410]     GUARD rec_0500 == 1
[0417]     ENDIF
[0418]     SAY "..."  '[voice 1]
[0422]     SAY "Another broken-down robot , Commander ... This planet's a gold mine ..."
[0442]     SAY "Let's teleport it , Commander . You never know when these things can come in handy ..."
[046C]     SAY "TELEPORT ROBOT TO ARK ... word_65535 teleport"
[0484]     IF-BLOCK (exit -> @04A4)
[0487]       GUARD concept == "teleport"
[048A]       ENDIF
[048B]       SAY "TELEPORTING ROBOT TO CRYOBOX"  '[skip 3]
[049B]       rec_04E2 = 65535
[04A0]       CLEAR concept_alt
[04A1]       END PRESENTATION Anna_Haf.talk
    END
  END
[04A4]   BLOCK (exit -> @04D6)
[04A8]     AWAIT gameflag_274F
[04A9]     GUARD active_actor == Anna_Haf.talk (related 40)
[04AE]     ENDIF
[04AF]     SAY "Commander , this is one tough repair job ..."
[04C9]     SAY "stop"  '[skip 1]
[04D3]     END PRESENTATION Anna_Haf.talk
  END
[04D6]   BLOCK (exit -> @04E6)
[04DA]     GUARD NOT rec_02A2 == 2846
[04E0]     ENDIF
[04E1]     rec_0692 = 3638
  END
[04E6]   BLOCK (exit -> @05DB)
[04EA]     AWAIT gameflag_252A
[04EB]     GUARD A1 == 0
[04F2]     GUARD rec_1088 == 3608
[04F7]     GUARD active_actor == Izwalito.talk (related 40)
[04FC]     ENDIF
[04FD]     SAY "Izwalito happy to see you back , friend ..."  '[voice 2]
[0517]     SAY "You know bad news ?"  '[voice 0]
[0529]     SAY "Father of Yoko , Maxxon , be kidnapped ..."  '[voice 3]
[0543]     SAY "Kidnappers want big ransom , much CREDs ..."  '[voice 2]
[055B]     SAY "Me scared , friend . Me very worried . Me hide ..."  '[voice 5]
[057B]     SAY "You must help Yoko ... friend Commander ..."  '[voice 6]
[0593]     SAY "Me think kidnapper be Croolis escaped from Mastachok jail ..."  '[voice 2]
[05AF]     SAY "Me hide ... FEAR ... FEAR ..."  '[voice 4]
[05C5]     SAY "Bye bye ..."  '[voice 5, skip 3]
[05D3]     rec_0692 = 4070
[05D8]     END PRESENTATION Izwalito.talk
  END
[05DB]   BLOCK (exit -> @05EF)
[05DF]     GUARD kra2 == 1
[05E6]     ENDIF
[05E7]     state[10] = 200
[05EB]     POKE [0x05DC] = 0
  END
[05EF]   BLOCK (exit -> @05FF)
[05F3]     GUARD state[10] == 0
[05F5]     ENDIF
[05F6]     OP_C3 C3 24 06 28 00
[05FB]     POKE [0x05F0] = 0
  END
[05FF]   BLOCK (exit -> @06F6)
[0603]     AWAIT presentation
[0604]     GUARD active_actor == Kran_Dobu.talk (related 40)
[0609]     ENDIF
[060A]     SAY "This is Kran Dobu , Space Knight ..."
[0622]     SAY "Radio message to Ark ... Ha! Ha! I bet my Kraner IV flies faster than your clunker ..."
[064E]     SAY "Ha! Ha! Ha! Let's meet up at point X337 Y242 ..."
[066C]     SAY "I have a fix on his ship , Commander ..."
[0688]     SAY "Got the guts to race me ? ... Ha! Ha! Ha! ... I'm waiting ..."
[06AE]     SAY "Commander , I suggest we ignore him ..."
[06C6]     SAY "Chicken , huh ? ... Ha! Ha! Ha!"
[06DE]     SAY "Bye bye"  '[skip 3]
[06EA]     POKE [0x0600] = 0
[06EE]     rec_10F8 |= 0x2
[06F3]     END PRESENTATION Kran_Dobu.talk
  END
[06F6]   BLOCK (exit -> @0881)
[06FA]     AWAIT gameflag_252A
[06FB]     GUARD active_actor == Kran_Dobu.talk (related 40)
[0700]     ENDIF
[0701]     SAY "Ha! Ha! Ha!"  '[voice 1]
[070F]     SAY "What a heap of reject junk ... You mean that thing actually flies ? Ha! Ha! Ha!"  '[voice 2]
[0739]     SAY "You're not afraid ..."  '[voice 3]
[0749]     SAY "Okay , no more kidding ... Let's race ... The first who gets to , uh ... Let's see ..."  '[voice 4]
[0779]     SAY "To planet Troma ! Ha! Ha! Ha!"  '[voice 5]
[078F]     SAY "You have the coordinates ... Planet Troma is in the TROMUS constellation x432 , Y654 ..."  '[voice 6]
[07B7]     SAY "Commander , he's gonna get trashed ... There's no way he can match our suction turbos ..."
[07E1]     SAY "If you win , I'll give you my bionic guitar ... If I win , you'll give me something ..."  '[voice 0]
[0811]     SAY "On your marks , guy ..."  '[voice 7]
[0825]     SAY "Ready"  '[voice 7]
[082F]     SAY "Set"  '[voice 7]
[0839]     SAY "GO ..."  '[voice 7, skip 2]
[0845]     rec_110E.pair = (10, 10)
[084C]     LOADSTR "krando20.hnm"
[085B]     SAY "See you round , comrade ..."  '[voice 1, skip 4]
[086F]     rec_10F8 &= !0x2
[0875]     rec_0F70 |= 0x2
[087A]     POKE [0x06F7] = 0
[087E]     END PRESENTATION Kran_Dobu.talk
  END
[0881]   BLOCK (exit -> @0893)
[0885]     GUARD rec_1088 == 3950
[088A]     ENDIF
[088B]     state[11] = 100
[088F]     POKE [0x0882] = 0
  END
[0893]   BLOCK (exit -> @08A7)
[0897]     GUARD state[11] == 0
[0899]     ENDIF
[089A]     OP_C3 C3 24 06 28 00
[089F]     POKE [0x08A8] = 1
[08A3]     POKE [0x0894] = 0
  END
[08A7]   GOTO @09A9
[08AB]   AWAIT presentation
[08AC]   GUARD active_actor == Kran_Dobu.talk (related 40)
[08B1]   ENDIF
[08B2]   SAY "Ti ti ti , ta ta ta , ti ti ti ..."
[08D2]   SAY "Ti ti ti , ta ta ta , ti ti ti ..."
[08F2]   SAY "Commander , Commander ! It's an SOS , a distress call ..."
[0912]   SAY "S ... O ... S ... THIS IS KRANER ... IN TROUBLE ... BIG BREAKDOWN ..."
[093A]   SAY "POSITION X765 Y234 ... BREAKDOWN ... S ... O ... S ..."  '[skip 1]
[095A]   rec_10F8 |= 0x2
[095F]   SAY "Commander , it sounds like a breakdown ... We'd better help him ..."
[0981]   SAY "KRUIIIK ..."  '[skip 1]
[098D]   LOADSTR "krando20.hnm"
[099C]   SAY "..."  '[skip 1]
[09A6]   END PRESENTATION Kran_Dobu.talk
[09A9]   BLOCK (exit -> @0A6B)
[09AD]     AWAIT gameflag_252A
[09AE]     GUARD active_actor == Kran_Dobu.talk (related 40)
[09B3]     GUARD panne == 0
[09BA]     ENDIF
[09BB]     SAY "Ha! Hi guy . I was wondering when you'd show up ..."  '[voice 1]
[09DB]     SAY "My engine's totalled ... I must've pushed it too hard ... It just went : ARG ... PSHHHHHH !"  '[voice 2]
[0A09]     SAY "Think you can fix it for me ?"  '[voice 3]
[0A21]     SAY "Teleport Morning Oil over to him , Commander . He's an ace repairman ..."
[0A45]     SAY "I'm waiting , guy ..."  '[voice 4]
[0A57]     SAY "..."  '[skip 2]
[0A61]     panne = 1
[0A68]     END PRESENTATION Kran_Dobu.talk
  END
[0A6B]   BLOCK (exit -> @0C75)
[0A6F]     AWAIT gameflag_252A
[0A70]     GUARD active_actor == Kran_Dobu.talk (related 40)
[0A75]     ENDIF
[0A76]     IF-BLOCK (exit -> @0ACF)
[0A79]       GUARD panne == 1
[0A80]       GUARD rec_03C2 == 4246
[0A85]       ENDIF
[0A86]       SAY "I'm still waiting , old buddy . You're not gonna let me down now , huh ? ..."  '[voice 1]
[0AB2]       SAY "Commander , Morning Oil is an expert repairman ..."  '[skip 1]
[0ACC]       END PRESENTATION Kran_Dobu.talk
    END
[0ACF]     IF-BLOCK (exit -> @0B84)
[0AD2]       GUARD rec_03C2 == 4070
[0AD7]       GUARD panne == 1
[0ADE]       ENDIF
[0ADF]       SAY "Thanks , buddy . One heck of a robot you got here ... He's taken the whole thing to bits ... Ha! Ha! I love watching experts ..."  '[voice 3]
[0B1F]       SAY "Uh ... I hope he knows how to put everything back together ... I mean , anyone can take things apart ..."
[0B53]       SAY "Hey ! It works ! He got my engines working ... I am dazed and amazed !!!"  '[voice 4, skip 1]
[0B7D]       panne = 2
    END
[0B84]     IF-BLOCK (exit -> @0C0E)
[0B87]       GUARD rec_03C2 == 4070
[0B8C]       GUARD panne == 2
[0B93]       ENDIF
[0B94]       SAY "Thanks guys ... You are beautiful ..."  '[voice 3]
[0BAA]       SAY "Here's my bionic guitar , as promised ... You be careful with it ..."  '[voice 4]
[0BCE]       SAY "TELEPORT GUITAR TO ARK ... word_65535 teleport"
[0BE6]       IF-BLOCK (exit -> @0C0E)
[0BE9]         GUARD concept == "teleport"
[0BEC]         ENDIF
[0BED]         SAY "TELEPORTING GUITAR TO CRYOBOX ..."  '[skip 3]
[0BFF]         OP_CD CD 24 06 7E 13 28 00
[0C06]         panne = 3
[0C0D]         CLEAR concept_alt
      END
    END
[0C0E]     IF-BLOCK (exit -> @0C47)
[0C11]       GUARD rec_03C2 == 4070
[0C16]       GUARD panne == 3
[0C1D]       ENDIF
[0C1E]       SAY "Your robot is back on your ship ... He has something for you ..."  '[skip 1]
[0C42]       rec_03C2 = 65535
    END
[0C47]     SAY "See you round ... Buddy ..."  '[voice 4]
[0C5B]     SAY "..."  '[skip 3]
[0C65]     rec_10F8 &= !0x2
[0C6B]     rec_110E.pair = (100, 10)
[0C72]     END PRESENTATION Kran_Dobu.talk
  END
[0C75]   BLOCK (exit -> @0D76)
[0C79]     AWAIT gameflag_252A
[0C7A]     GUARD rec_03C2 == 4246
[0C7F]     GUARD B1 == 0
[0C86]     GUARD active_actor == Morning_Oil.talk (related 40)
[0C8B]     ENDIF
[0C8C]     SAY "I reprogrammed him , Commander . He's operational ..."
[0CA6]     SAY "Hello , Commander ..."
[0CB6]     SAY "I await your instructions ..."
[0CC8]     IF-BLOCK (exit -> @0D69)
[0CCB]       GUARD panne == 1
[0CD2]       GUARD rec_1088 == 4342
[0CD7]       ENDIF
[0CD8]       SAY "Teleport him over to the KRANER , Commander . He'll soon fix Mister Kran Dobu's breakdown ..."
[0D02]       SAY "TELEPORT MORNING OIL TO KRANER ... word_65535 teleport refuse"
[0D1E]       IF-BLOCK (exit -> @0D42)
[0D21]         GUARD concept == "teleport"
[0D24]         ENDIF
[0D25]         SAY "TELEPORTING MORNING OIL TO KRANER ..."  '[skip 3]
[0D39]         rec_03C2 = 4070
[0D3E]         CLEAR concept_alt
[0D3F]         END PRESENTATION Morning_Oil.talk
      END
[0D42]       IF-BLOCK (exit -> @0D69)
[0D45]         GUARD concept == "refuse"
[0D48]         ENDIF
[0D49]         SAY "OK , Commander . Your wish is my command ..."  '[skip 2]
[0D65]         CLEAR concept_alt
[0D66]         END PRESENTATION Morning_Oil.talk
      END
    END
[0D69]     SAY "..."  '[skip 1]
[0D73]     END PRESENTATION Morning_Oil.talk
  END
[0D76]   BLOCK (exit -> @0E7A)
[0D7A]     AWAIT gameflag_274F
[0D7B]     GUARD active_actor == Morning_Oil.talk (related 40)
[0D80]     ENDIF
[0D81]     IF-BLOCK (exit -> @0E31)
[0D84]       GUARD panne == 3
[0D8B]       GUARD B1 == 0
[0D92]       ENDIF
[0D93]       SAY "Commander , I have repaired Mister Kran Dobu's vessel ..."  '[voice 3]
[0DAF]       SAY "He gave me two trasmitter receiver KEY RINGS . They're in the cryobox ..."  '[voice 4, skip 2]
[0DD3]       rec_025A = 65535
[0DD8]       bronk4 = 65535
[0DDD]       SAY "How about that ... Very weird key rings , believe me ... What in the name of Pete are they for ? ..."
[0E13]       SAY "Happy now , Commander ? ..."  '[skip 2]
[0E27]       B1 = 1
[0E2E]       END PRESENTATION Morning_Oil.talk
    END
[0E31]     SAY "I hear and obey , Commander ..."
[0E47]     SAY "Don't you love the way I programmed him ... Total docility is his watchword ..."
[0E6D]     SAY "..."  '[skip 1]
[0E77]     END PRESENTATION Morning_Oil.talk
  END
[0E7A]   BLOCK (exit -> @1141)
[0E7E]     AWAIT gameflag_274F
[0E7F]     GUARD NOT bronk4 == 1082
[0E85]     GUARD F1 == 0
[0E8C]     GUARD NOT rec_1088 == 2684
[0E92]     GUARD active_actor == Bronko.talk (related 40)
[0E97]     ENDIF
[0E98]     SAY "Commander , I'm getting rusty ..."  '[voice 2]
[0EAC]     IF-BLOCK (exit -> @0F2D)
[0EAF]       GUARD rec_0548 == 0
[0EB6]       GUARD rec_0470 < 2
[0EBD]       ENDIF
[0EBE]       SAY "Commander, Mister Bronko spoke to me of a musician friend of his who lives at the airport on planet Moskito..."
[0EEE]       SAY "True , Commander . A very fine musician ..."  '[voice 3]
[0F08]       SAY "If you have the time , you should check him out ..."  '[voice 4, skip 1]
[0F28]       rec_103C |= 0x2
    END
[0F2D]     IF-BLOCK (exit -> @1058)
[0F30]       GUARD rec_0470 == 1
[0F37]       ENDIF
[0F38]       SAY "Honk taught me your language . He's so patient ... Thanks HONK !"  '[voice 3]
[0F5A]       SAY "My pleasure , Mister Bronko . You're a talented student ..."
[0F78]       SAY "You're just saying that , Mister Honk ..."  '[voice 4]
[0F90]       SAY "No , you have great gifts ... Truly ..."
[0FAA]       SAY "You're making me blush now ... And butchers don't blush easily , you know ! Ha! Ha! Ha!"  '[voice 5]
[0FD6]       SAY "Ha! Ha! Ha! What a fine wit you have , Mister Bronko ! Ha! Ha! Ha!"
[0FFE]       SAY "This is one fine robot , Commander ..."
[1016]       SAY "As I was saying , Commander , nothing would please me more than to serve you ..."  '[voice 6]
[1040]       SAY "I could undertake a mission , maybe ..."  '[voice 7]
    END
[1058]     IF-BLOCK (exit -> @10D2)
[105B]       GUARD rec_0470 > 1
[1062]       GUARD rec_0590 == 0
[1069]       ENDIF
[106A]       SAY "I was pointing out , Commander , that I'd enjoy nothing better than to serve you ..."  '[voice 0]
[1094]       SAY "I could undertake a mission of observation at that unusual clinic ..."  '[voice 1]
[10B4]       SAY "We ought to go see the Gluxx family on planet EKATOMB..."
    END
[10D2]     IF-BLOCK (exit -> @1108)
[10D5]       GUARD rec_0470 > 2
[10DC]       GUARD rec_0590 > 0
[10E3]       GUARD NOT rec_1088 == 2684
[10E9]       ENDIF
[10EA]       SAY "Better approach the planet Erazor and offer Mister Bronko a mission..."
    END
[1108]     SAY "If you need me , don't hesitate a second ..."  '[voice 3]
[1124]     SAY "See you soon ..."  '[voice 5]
[1134]     SAY "..."  '[skip 1]
[113E]     END PRESENTATION Bronko.talk
  END
[1141]   BLOCK (exit -> @12EF)
[1145]     AWAIT gameflag_274F
[1146]     GUARD rec_1088 == 2684
[114B]     GUARD NOT bronk4 == 1082
[1151]     GUARD active_actor == Bronko.talk (related 40)
[1156]     ENDIF
[1157]     SAY "Commander , I'm getting bored in a big big way ... I need action ..."  '[voice 3]
[117D]     IF-BLOCK (exit -> @11FE)
[1180]       GUARD rec_0548 == 0
[1187]       GUARD rec_0470 < 2
[118E]       ENDIF
[118F]       SAY "Commander, Mister Bronko spoke to me of a musician friend of his who lives at the airport on planet Moskito..."
[11BF]       SAY "True , Commander . A very fine musician ..."  '[voice 3]
[11D9]       SAY "If you have the time , you should check him out ..."  '[voice 4, skip 1]
[11F9]       rec_103C |= 0x2
    END
[11FE]     IF-BLOCK (exit -> @12E2)
[1201]       GUARD rec_0590 > 0
[1208]       ENDIF
[1209]       SAY "Commander , why don't we send Mister Bronko to see what's happening on planet Erazor ..."
[1231]       SAY "Yes , Commander , that's a great idea . I'm sure there's a connection with the disappearance of the Gluxx kids ..."  '[voice 2]
[1265]       SAY "Let's teleport him , Commander . He'll keep us informed by radio ..."
[1287]       SAY "TELEPORT BRONKO TO ERAZOR word_65535 YES NO"
[129F]       IF-BLOCK (exit -> @12C6)
[12A2]         GUARD concept == "YES"
[12A5]         ENDIF
[12A6]         SAY "TELEPORTING BRONKO TO ERAZOR"  '[skip 4]
[12B6]         brk = 1
[12BD]         rec_0452 = 2684
[12C2]         CLEAR concept_alt
[12C3]         END PRESENTATION Bronko.talk
      END
[12C6]       IF-BLOCK (exit -> @12E2)
[12C9]         GUARD concept == "NO"
[12CC]         ENDIF
[12CD]         SAY "As you wish , Commander ..."  '[skip 1]
[12E1]         CLEAR concept_alt
      END
    END
[12E2]     SAY "..."  '[skip 1]
[12EC]     END PRESENTATION Bronko.talk
  END
[12EF]   BLOCK (exit -> @13A9)
[12F3]     AWAIT gameflag_252A
[12F4]     GUARD rec_1088 == 2684
[12F9]     GUARD active_actor == Bronko.talk (related 40)
[12FE]     ENDIF
[12FF]     SAY "Commander , There's nobody here ..."  '[voice 5]
[1313]     SAY "I'll look around , Commander ... Use the phone to call me ..."  '[voice 2]
[1335]     SAY "Good luck , Mister Bronko . I admire the way you get things done ..."
[135B]     SAY "Shucks , Mister Honk ... It just the way nature made me ..."  '[voice 6]
[137D]     SAY "Nature . What a wonderful invention ..."
[1393]     SAY "..."  '[skip 3]
[139D]     rec_043C |= 0x2
[13A2]     POKE [0x12F0] = 0
[13A6]     END PRESENTATION Bronko.talk
  END
[13A9]   BLOCK (exit -> @15F5)
[13AD]     AWAIT presentation
[13AE]     GUARD rec_0452 == 2684
[13B3]     GUARD active_actor == Bronko.talk (related 40)
[13B8]     ENDIF
[13B9]     IF-BLOCK (exit -> @142E)
[13BC]       GUARD NOT rec_1088 == 2684
[13C2]       ENDIF
[13C3]       SAY "Krrrr Bzzz This is Bronko .... Bzzzzz I can't hear you .... Krrrk ..."
[13E7]       SAY "kkkkKrouik..."
[13F1]       SAY "He's having auditory difficulties , Commander . He's too far away ... We ought to get closer to planet Erazor..."
[1421]       SAY "..."  '[skip 1]
[142B]       END PRESENTATION Bronko.talk
    END
[142E]     IF-BLOCK (exit -> @15F5)
[1431]       GUARD rec_1088 == 2684
[1436]       ENDIF
[1437]       SAY "Yes , hello ... Commander ... Bronko here ... I'm receiving you loud and clear ..."
[145F]       SAY "There are quite a few Slimer ships coming and going . Doctor Otto Von Smile appears to have left ..."  '[skip 1]
[148F]       rec_0480 = 12
[1494]       IF-BLOCK (exit -> @15BA)
[1497]         GUARD bronk4 == 65535
[149C]         ENDIF
[149D]         SAY "Commander , how about teleporting one of those spy keyrings to Mister Bronko ? He could hide it in Doctor Otto Von Smile's desk ..."
[14D7]         SAY "That way we could listen to whatever he's doing ..."
[14F3]         SAY "It's a great idea , Commander ..."
[1509]         SAY "Oh , you flatter me , Mister Bronko . Any fool would have thought of it ..."
[1533]         SAY "No , no . It's an idea of rare perspicacity ..."
[1551]         SAY "How can I say this , Commander ... Mister Bronko is so... so ... admirable ! ..."
[157B]         SAY "TELEPORT TRANSMITTER KEYRING TO BRONKO ... word_65535 teleport"
[1595]         IF-BLOCK (exit -> @15BA)
[1598]           GUARD concept == "teleport"
[159B]           ENDIF
[159C]           SAY "TELEPORTING SPY KEYRING TRANSMITTER TO BRONKO ..."  '[skip 2]
[15B2]           OP_CD CD 30 00 96 13 3A 04
[15B9]           CLEAR concept_alt
        END
      END
[15BA]       SAY "See you soon , Commander ..."
[15CE]       SAY "Bronko is so brave , isn't he ? ..."
[15E8]       SAY "..."  '[skip 1]
[15F2]       END PRESENTATION Bronko.talk
    END
  END
[15F5]   BLOCK (exit -> @1723)
[15F9]     AWAIT gameflag_252A
[15FA]     GUARD rec_1088 == 2684
[15FF]     GUARD rec_0452 == 2684
[1604]     GUARD active_actor == Bronko.talk (related 40)
[1609]     ENDIF
[160A]     IF-BLOCK (exit -> @167B)
[160D]       GUARD NOT bronk4 == 1082
[1613]       ENDIF
[1614]       SAY "What are you doing , Commander ? ... You're going to get us spotted ..."  '[voice 1]
[163A]       SAY "Use the phone to contact me ..."  '[voice 2]
[1650]       SAY "He's right , Commander . You'll get him spotted ! ..."
[166E]       SAY "..."  '[skip 1]
[1678]       END PRESENTATION Bronko.talk
    END
[167B]     IF-BLOCK (exit -> @1723)
[167E]       GUARD bronk4 == 1082
[1683]       ENDIF
[1684]       SAY "Commander , I hid the spy keyring in Doctor Otto Von Smile's desk ..."  '[voice 2]
[16A8]       SAY "I have completed my mission . Teleport me back to the Ark ..."  '[voice 5]
[16CA]       SAY "Great work , Mister Bronko ! ..."
[16E0]       SAY "TELEPORT BRONKO TO CRYOBOX word_65535 teleport"
[16F6]       IF-BLOCK (exit -> @1723)
[16F9]         GUARD concept == "teleport"
[16FC]         ENDIF
[16FD]         SAY "TELEPORTING BRONKO TO CRYOBOX"  '[skip 5]
[170D]         rec_0452 = 65535
[1712]         vari = 1
[1719]         rec_043C &= !0x2
[171F]         CLEAR concept_alt
[1720]         END PRESENTATION Bronko.talk
      END
    END
  END
[1723]   BLOCK (exit -> @1879)
[1727]     AWAIT gameflag_274F
[1728]     GUARD bronk4 == 1082
[172D]     GUARD active_actor == Bronko.talk (related 40)
[1732]     ENDIF
[1733]     SAY "Phew ... I'm happy to be back ..."  '[voice 5]
[174B]     SAY "I'm happy to see you back , Mister Bronko ... I was so anxious about you ..."
[1775]     IF-BLOCK (exit -> @17C1)
[1778]       GUARD NOT rec_02A2 == 2846
[177E]       ENDIF
[177F]       SAY "Take a look in the cryobox , Commander . I brought something back for you ..."  '[voice 5]
[17A7]       SAY "..."  '[skip 4]
[17B1]       rec_131A = 65535
[17B6]       POKE [0x187A] = 1
[17BA]       POKE [0x1724] = 0
[17BE]       END PRESENTATION Bronko.talk
    END
[17C1]     IF-BLOCK (exit -> @1842)
[17C4]       GUARD rec_0548 == 0
[17CB]       GUARD rec_0470 < 2
[17D2]       ENDIF
[17D3]       SAY "Commander, Mister Bronko spoke to me of a musician friend of his who lives at the airport on planet Moskito..."
[1803]       SAY "True , Commander . A very fine musician ..."  '[voice 3]
[181D]       SAY "If you have the time , you should check him out ..."  '[voice 4, skip 1]
[183D]       rec_103C |= 0x2
    END
[1842]     SAY "That Mister Bronko... What a character"
[1856]     SAY "Well , I'll just get back to my cooking , Commander ..."  '[skip 1]
[1876]     END PRESENTATION Bronko.talk
  END
[1879]   GOTO @1936
[187D]   AWAIT gameflag_274F
[187E]   GUARD active_actor == Bronko.talk (related 40)
[1883]   ENDIF
[1884]   SAY "Everything's under control , Commander ..."  '[voice 5]
[1898]   IF-BLOCK (exit -> @1919)
[189B]     GUARD rec_0548 == 0
[18A2]     GUARD rec_0470 < 2
[18A9]     ENDIF
[18AA]     SAY "Commander, Mister Bronko spoke to me of a musician friend of his who lives at the airport on planet Moskito..."
[18DA]     SAY "True , Commander . A very fine musician ..."  '[voice 3]
[18F4]     SAY "If you have the time , you should check him out ..."  '[voice 4, skip 1]
[1914]     rec_103C |= 0x2
  END
[1919]   SAY "I better get back to the kitchen , Commander..."  '[voice 4, skip 1]
[1933]   END PRESENTATION Bronko.talk
[1936]   GOTO @1973
[193A]   AWAIT gameflag_274F
[193B]   GUARD active_actor == receiver.talk (related 40)
[1940]   GUARD state[2] == 0
[1942]   GUARD bronk4 == 1082
[1947]   ENDIF
[1948]   SAY "SHHHHHHHHHHHHHHHHHHHHHHHHHHH ..."
[1954]   SAY "Still nothing , Commander ..."
[1966]   SAY "..."  '[skip 1]
[1970]   END PRESENTATION receiver.talk
[1973]   BLOCK (exit -> @1A70)
[1977]     AWAIT gameflag_274F
[1978]     GUARD active_actor == receiver.talk (related 40)
[197D]     GUARD vari == 1
[1984]     GUARD bronk4 == 1082
[1989]     ENDIF
[198A]     SAY "Quick ... Hurry it up ... We're taking everything ..."
[19A6]     SAY "Don't forget food for the prisoners ... Come on , you're too slow ..."
[19CA]     SAY "Or else you'll get a jab from the doctor, and you'd hate it , believe me ... Ha! Ha! Ha!"
[19FA]     SAY "The first shipment can take off for the secret base ..."
[1A18]     SAY "The guinea-pig prisoners will go to the secret jail ..."
[1A34]     SAY "It's unbelievable , Commander ..."
[1A46]     SAY "Hurry ... Faster ! ..."
[1A58]     SAY "..."  '[skip 3]
[1A62]     fish = 1
[1A69]     POKE [0x1974] = 0
[1A6D]     END PRESENTATION receiver.talk
  END
[1A70]   BLOCK (exit -> @1AA8)
[1A74]     AWAIT gameflag_274F
[1A75]     GUARD active_actor == receiver.talk (related 40)
[1A7A]     ENDIF
[1A7B]     SAY "BZZZZZ ..."
[1A87]     SAY "BEEP ... BEEP ... BEEP ..."
[1A9B]     SAY "..."  '[skip 1]
[1AA5]     END PRESENTATION receiver.talk
  END
[1AA8]   BLOCK (exit -> @1E53)
[1AAC]     AWAIT gameflag_252A
[1AAD]     GUARD NOT rec_116A == 40
[1AB3]     GUARD rec_1088 == 3056
[1AB8]     GUARD G1 == 0
[1ABF]     GUARD active_actor == Hom.talk (related 40)
[1AC4]     ENDIF
[1AC5]     rec_0318 = 12
[1ACA]     SAY "Welcome , stranger ..."  '[voice 2]
[1ADA]     SAY "You be great traveller ..."  '[voice 3]
[1AEC]     IF-BLOCK (exit -> @1C00)
[1AEF]       GUARD rec_0308 == 1
[1AF6]       ENDIF
[1AF7]       SAY "WOWEE !! Commander, he's even uglier than ol' Turkeyface Bob ... Ha ! Ha ! Ha !"
[1B21]       SAY "Me HOM . Me big tube brain . Me delighted receive you , Commander ..."  '[voice 2]
[1B47]       SAY "Me catch your print , friend ... Me read TRUTH in print ..."  '[voice 2]
[1B69]       SAY "You click hard , friend . Me catch your print on mouse button ... word_65535 click refuse"  '[voice 3]
[1B95]       IF-BLOCK (exit -> @1BB1)
[1B98]         GUARD concept == "click"
[1B9B]         ENDIF
[1B9C]         SAY "You not click hard enough ..."  '[voice 3, skip 1]
[1BB0]         CLEAR concept_alt
      END
[1BB1]       IF-BLOCK (exit -> @1C00)
[1BB4]         GUARD concept == "refuse"
[1BB7]         ENDIF
[1BB8]         SAY "You refuse ... Me not like that ... Me suspect you ..."  '[voice 4]
[1BD8]         SAY "Bye bye ... Me not like you refuse ..."  '[voice 5]
[1BF2]         SAY "..."  '[voice 2, skip 2]
[1BFC]         CLEAR concept_alt
[1BFD]         END PRESENTATION Hom.talk
      END
    END
[1C00]     SAY "You click very very hard , friend . Me catch print on mouse ... word_65535 click"  '[voice 3]
[1C2A]     IF-BLOCK (exit -> @1C61)
[1C2D]       GUARD concept == "click"
[1C30]       ENDIF
[1C31]       SAY "Ahh ! Me catch print ... Ohh , you nice print ..."  '[voice 3, skip 2]
[1C51]       LOADSTR "scandoig.hnm"
[1C60]       CLEAR concept_alt
    END
[1C61]     SAY "Ooooouh! Me see you , friend ... You have pretty box with picture of Hom inside ..."  '[voice 6, skip 1]
[1C8B]     LOADSTR "kort.hnm"
[1C96]     SAY "Me read prints . You have little mouse in hand with long long tail ..."  '[voice 7]
[1CBC]     SAY "Me see keyboard ... And oh ! CDROM double speed . Nice hard disk ... You be lucky , friend ..."  '[voice 8]
[1CEE]     SAY "Welcome to my planet , stranger ..."  '[voice 4]
[1D04]     SAY "You help my Izwal friends from planet Corpo . You be nice ... You must join GUILD OF MEMBERS"  '[voice 9]
[1D32]     SAY "GUILD OF MEMBERS be big cybernetic organization ..."  '[voice 10]
[1D4A]     SAY "To be in GUILD OF MEMBERS you must be initiated ..."  '[voice 3]
[1D68]     SAY "You know universe . You must take exam ..."  '[voice 5]
[1D82]     SAY "You go to planet CYBEROCK where be cyber university ..."  '[voice 4]
[1D9E]     SAY "You take cyber studies , friend ..."  '[voice 4]
[1DB4]     SAY "And you take cyber exam to be D.O.R.K. , Doctor of Rare Knowledge ..."  '[voice 2]
[1DD8]     SAY "Cyberock be in Cyberius system , coordinates X234 Y546 ..."  '[voice 5, skip 1]
[1DF4]     rec_0DFC |= 0x2
[1DF9]     SAY "And one more planet to visit ! We're going places ..."
[1E17]     SAY "Me wait for you with D.O.R.K. Doctor of Rare Knowledge diploma ..."  '[voice 6]
[1E37]     SAY "Bye bye , friend ..."  '[skip 2]
[1E49]     G1 = 1
[1E50]     END PRESENTATION Hom.talk
  END
[1E53]   BLOCK (exit -> @203A)
[1E57]     AWAIT gameflag_252A
[1E58]     GUARD G1 == 1
[1E5F]     GUARD active_actor == Hom.talk (related 40)
[1E64]     GUARD rec_1088 == 3056
[1E69]     ENDIF
[1E6A]     SAY "Welcome , friend ..."  '[voice 2]
[1E7A]     SAY "WOW !! I can't believe that guy's head , Commander . Yukkee ... Ha ! Ha ! Ha !"
[1EA8]     SAY "Me HOM . me big tube brain. Me happy to see you , Commander ..."  '[voice 2]
[1ECE]     SAY "Me catch your print , friend ... Me again read your print ..."  '[voice 2]
[1EF0]     SAY "You click hard , friend . Me catch your print on mouse button ... word_65535 click refuse"  '[voice 3]
[1F1C]     IF-BLOCK (exit -> @1F47)
[1F1F]       GUARD concept == "click"
[1F22]       ENDIF
[1F23]       SAY "Me recognize you , friend ..."  '[voice 3, skip 2]
[1F37]       LOADSTR "scandoig.hnm"
[1F46]       CLEAR concept_alt
    END
[1F47]     IF-BLOCK (exit -> @1F96)
[1F4A]       GUARD concept == "refuse"
[1F4D]       ENDIF
[1F4E]       SAY "You refuse ... Me not like that ... Me suspect you ..."  '[voice 4]
[1F6E]       SAY "Bye bye ... Me not like you refuse ..."  '[voice 5]
[1F88]       SAY "..."  '[skip 2]
[1F92]       CLEAR concept_alt
[1F93]       END PRESENTATION Hom.talk
    END
[1F96]     SAY "Welcome to planet KORTEX , friend ..."  '[voice 4]
[1FAC]     SAY "GUILD OF MEMBERS be big cybernetic organization ..."  '[voice 10]
[1FC4]     IF-BLOCK (exit -> @2029)
[1FC7]       GUARD NOT rec_116A == 40
[1FCD]       ENDIF
[1FCE]       SAY "You must take DORK exam on planet Cyberock , friend ..."  '[voice 2]
[1FEC]       SAY "You come back if you take DORK exam ..."  '[voice 3]
[2006]       SAY "Go now , friend . GUILD OF MEMBERS count on you ..."  '[voice 2, skip 1]
[2026]       END PRESENTATION Hom.talk
    END
[2029]     SAY "..."  '[skip 1]
[2033]     G1 = 2
  END
[203A]   BLOCK (exit -> @217F)
[203E]     AWAIT gameflag_252A
[203F]     GUARD G1 == 2
[2046]     GUARD active_actor == Hom.talk (related 40)
[204B]     ENDIF
[204C]     SAY "Me meditate ..."  '[voice 2]
[205A]     SAY "ATA ATA HOGLO HULU..."  '[voice 5]
[206A]     IF-BLOCK (exit -> @217F)
[206D]       GUARD rec_116A == 40
[2072]       ENDIF
[2073]       SAY "Well done ... Friend , you pass DORK . You be cyber intelligent ..."  '[voice 3]
[2097]       SAY "Me reward you . GUILD OF MEMBERS give you brain scrambler ..."  '[voice 4]
[20B7]       SAY "A what scrambler ???"
[20C7]       SAY "Brain scrambler be powerful weapon ..."  '[voice 5]
[20DB]       SAY "TELEPORT BRAIN SCRAMBLER TO CRYOBOX word_65535 teleport refuse"
[20F5]       IF-BLOCK (exit -> @2116)
[20F8]         GUARD concept == "teleport"
[20FB]         ENDIF
[20FC]         SAY "TELEPORTING BRAIN SCRAMBLER TO CRYOBOX"  '[skip 2]
[210E]         OP_CD CD 0C 03 AE 13 28 00
[2115]         CLEAR concept_alt
      END
[2116]       IF-BLOCK (exit -> @213F)
[2119]         GUARD concept == "refuse"
[211C]         ENDIF
[211D]         SAY "YOU CANNOT REFUSE . YOUR BRAIN IS SCRAMBLED ..."  '[skip 2]
[2137]         OP_CD CD 0C 03 AE 13 28 00
[213E]         CLEAR concept_alt
      END
[213F]       SAY "Me concentrate ..."  '[voice 2]
[214D]       SAY "Me vanish in vertiginiously infinite cosmic space ..."  '[voice 3]
[2165]       SAY "Bye bye , friend ..."  '[skip 2]
[2177]       rec_02EA = 4070
[217C]       END PRESENTATION Hom.talk
    END
  END
[217F]   BLOCK (exit -> @21D3)
[2183]     AWAIT gameflag_252A
[2184]     GUARD R1 == 0
[218B]     GUARD active_actor == Cyberquizz.talk (related 40)
[2190]     GUARD NOT rec_116A == 866
[2196]     GUARD rec_1088 == 3578
[219B]     ENDIF
[219C]     SAY "Bye bye , student Commander Blood . You are a member of the "GUILD OF MEMBERS" ..."  '[voice 1]
[21C6]     SAY "..."  '[voice 2, skip 1]
[21D0]     END PRESENTATION Cyberquizz.talk
  END
[21D3]   BLOCK (exit -> @2278)
[21D7]     AWAIT gameflag_252A
[21D8]     GUARD R1 == 0
[21DF]     GUARD active_actor == Cyberquizz.talk (related 40)
[21E4]     GUARD rec_1088 == 3578
[21E9]     ENDIF
[21EA]     POKE [0x2527] = 1
[21EE]     POKE [0x25DA] = 1
[21F2]     POKE [0x2683] = 1
[21F6]     POKE [0x2744] = 1
[21FA]     POKE [0x27E5] = 1
[21FE]     POKE [0x288C] = 1
[2202]     POKE [0x2939] = 1
[2206]     POKE [0x29D4] = 1
[220A]     POKE [0x2A6F] = 1
[220E]     POKE [0x2B0A] = 1
[2212]     POKE [0x2BAD] = 1
[2216]     POKE [0x2C3E] = 1
[221A]     POKE [0x2CD1] = 1
[221E]     POKE [0x2D66] = 1
[2222]     POKE [0x2DFB] = 1
[2226]     POKE [0x2E94] = 1
[222A]     POKE [0x2F2B] = 1
[222E]     POKE [0x2FD4] = 1
[2232]     POKE [0x306D] = 1
[2236]     POKE [0x30FE] = 1
[223A]     POKE [0x31A7] = 1
[223E]     POKE [0x323E] = 1
[2242]     POKE [0x32D1] = 1
[2246]     POKE [0x3372] = 1
[224A]     POKE [0x33F3] = 1
[224E]     POKE [0x3480] = 1
[2252]     POKE [0x351D] = 1
[2256]     POKE [0x35B2] = 1
[225A]     POKE [0x3655] = 1
[225E]     POKE [0x36EE] = 1
[2262]     POKE [0x3787] = 1
[2266]     POKE [0x3818] = 1
[226A]     quest = 0
[2271]     Bof = 0
  END
[2278]   BLOCK (exit -> @2369)
[227C]     AWAIT gameflag_252A
[227D]     GUARD R1 == 0
[2284]     GUARD active_actor == Cyberquizz.talk (related 40)
[2289]     GUARD rec_1088 == 3578
[228E]     ENDIF
[228F]     SAY "Student Commander Blood , today marks a turning point in your life ..."  '[voice 1]
[22B1]     SAY "Holy sombreros , Commander . It's ugly ! ..."
[22CB]     SAY "I shall be your DORK examiner . You can become a Doctor Of Rare Knowledge ..."  '[voice 2]
[22F3]     SAY "Dummy ! At your age I was already a NERD , a Notarized Expert in Rare Doctorates ..."  '[voice 3]
[231F]     SAY "Don't take any notice , Commander . Anything that repulsive can't be a real NERD ..."
[2347]     SAY "Here are the questions ... Are you ready ?"  '[voice 4, skip 2]
[2361]     POKE [0x21D4] = 0
[2365]     POKE [0x2279] = 0
  END
[2369]   BLOCK (exit -> @2471)
[236D]     AWAIT gameflag_252A
[236E]     GUARD active_actor == Cyberquizz.talk (related 40)
[2373]     GUARD Bof == 5
[237A]     ENDIF
[237B]     SAY "I hereby declare you a DORK !"  '[voice 1, skip 1]
[2391]     LOADSTR "cadeaux.hnm"
[239F]     SAY "YES !!!! WAY TO GO , COMMANDER . I KNEW YOU HAD IT IN YOU ..."
[23C7]     SAY "Here is your official DORK diploma ..."  '[voice 1]
[23DD]     SAY "TELEPORT DIPLOMA TO ARK word_65535 teleport"
[23F3]     IF-BLOCK (exit -> @2414)
[23F6]       GUARD concept == "teleport"
[23F9]       ENDIF
[23FA]       SAY "TELEPORTING DORK DIPLOMA TO ARK"  '[skip 2]
[240C]       OP_CD CD 9C 03 56 11 28 00
[2413]       CLEAR concept_alt
    END
[2414]     SAY "You are now ready to pass the S.U.C.K.E.R. exam , to become a Specialised Universal Cosmic Knowledge Expert Recognizer ..."  '[voice 3]
[2444]     SAY "See you soon , Commander..."  '[voice 2]
[2456]     SAY "stop"  '[skip 3]
[2460]     Bof = 0
[2467]     quest = 0
[246E]     END PRESENTATION Cyberquizz.talk
  END
[2471]   BLOCK (exit -> @2526)
[2475]     AWAIT gameflag_252A
[2476]     GUARD active_actor == Cyberquizz.talk (related 40)
[247B]     GUARD quest == 32
[2482]     GUARD Bof < 10
[2489]     ENDIF
[248A]     SAY "You failed !"  '[voice 5]
[2498]     SAY "I knew you'd foul up , Commander . How could you be such an airhead ? ..."
[24C2]     SAY "May oxydised yellow liver reduce you to suppurating pustules ! ..."  '[voice 4, skip 1]
[24E0]     LOADSTR "maledict.hnm"
[24EF]     SAY "That's the way it goes ...."
[2503]     SAY "stop"  '[skip 5]
[250D]     POKE [0x2279] = 1
[2511]     POKE [0x21D4] = 1
[2515]     quest = 0
[251C]     Bof = 0
[2523]     END PRESENTATION Cyberquizz.talk
  END
[2526]   BLOCK (exit -> @25D9)
[252A]     AWAIT gameflag_252A
[252B]     GUARD active_actor == Cyberquizz.talk (related 40)
[2530]     ENDIF
[2531]     SAY "Name one satelite of the Oddland black hole ... word_65535 pterra proxima magnus ondoya"  '[voice 9]
[2557]     IF-BLOCK (exit -> @258D)
[255A]       GUARD concept == "pterra"
[255D]       ENDIF
[255E]       SAY "Pterra, of course. You've been studying your black holes , Commander ..."  '[voice 2, skip 3]
[257E]       Bof += 1
[2585]       quest += 1
[258C]       CLEAR concept_alt
    END
[258D]     IF-BLOCK (exit -> @25C7)
[2590]       GUARD NOT concept == "pterra"
[2594]       ENDIF
[2595]       SAY "Not so hot on satellites , eh , Commander ? Maybe you're not ready for this ."  '[voice 5, skip 2]
[25BF]       quest += 1
[25C6]       CLEAR concept_alt
    END
[25C7]     SAY "Next question :"  '[voice 3, skip 1]
[25D5]     POKE [0x2527] = 0
  END
[25D9]   BLOCK (exit -> @2682)
[25DD]     AWAIT gameflag_252A
[25DE]     GUARD active_actor == Cyberquizz.talk (related 40)
[25E3]     ENDIF
[25E4]     SAY "Which great Slimer is buried on the planet Vista? word_65535 gluxx gelati gumy yolk"  '[voice 6]
[260A]     IF-BLOCK (exit -> @2642)
[260D]       GUARD concept == "yolk"
[2610]       ENDIF
[2611]       SAY "Right ! The Great Yolk, our inspiration ... Hare Yolk. Hare Hare ..."  '[voice 7, skip 3]
[2633]       Bof += 1
[263A]       quest += 1
[2641]       CLEAR concept_alt
    END
[2642]     IF-BLOCK (exit -> @2670)
[2645]       GUARD NOT concept == "yolk"
[2649]       ENDIF
[264A]       SAY "I said Vista ! Stop dreaming about ondoyants and concentrate ..."  '[voice 8, skip 2]
[2668]       quest += 1
[266F]       CLEAR concept_alt
    END
[2670]     SAY "Next question :"  '[voice 3, skip 1]
[267E]     POKE [0x25DA] = 0
  END
[2682]   BLOCK (exit -> @2743)
[2686]     AWAIT gameflag_252A
[2687]     GUARD active_actor == Cyberquizz.talk (related 40)
[268C]     ENDIF
[268D]     SAY "Very well . Think carefully ..."  '[voice 5]
[26A1]     SAY "One plus one equals what ? word_65535 two three four twelve"  '[voice 1]
[26C1]     IF-BLOCK (exit -> @26F5)
[26C4]       GUARD concept == "three"
[26C7]       ENDIF
[26C8]       SAY "Yes indeed . Such is the marvel of cybernetic reproduction ..."  '[voice 4, skip 3]
[26E6]       Bof += 1
[26ED]       quest += 1
[26F4]       CLEAR concept_alt
    END
[26F5]     IF-BLOCK (exit -> @2731)
[26F8]       GUARD NOT concept == "three"
[26FC]       ENDIF
[26FD]       SAY "You mean you know nothing of cybernetic reproduction ? That's a flabby brain you have there , Commander..."  '[voice 3, skip 2]
[2729]       quest += 1
[2730]       CLEAR concept_alt
    END
[2731]     SAY "Next question :"  '[voice 3, skip 1]
[273F]     POKE [0x2683] = 0
  END
[2743]   BLOCK (exit -> @27E4)
[2747]     AWAIT gameflag_252A
[2748]     GUARD active_actor == Cyberquizz.talk (related 40)
[274D]     ENDIF
[274E]     SAY "Which Croolis triggered the Universal War ? word_65535 emasculator eviscerator lord_krater lord_sirtaki alig_athor"  '[voice 0]
[2772]     IF-BLOCK (exit -> @27A0)
[2775]       GUARD concept == "lord_krater"
[2778]       ENDIF
[2779]       SAY "That wasn't one of his best ideas ..."  '[voice 9, skip 3]
[2791]       Bof += 1
[2798]       quest += 1
[279F]       CLEAR concept_alt
    END
[27A0]     IF-BLOCK (exit -> @27D2)
[27A3]       GUARD NOT concept == "lord_krater"
[27A7]       ENDIF
[27A8]       SAY "Dummy ! It starts with a K and ends with another letter ..."  '[voice 8, skip 2]
[27CA]       quest += 1
[27D1]       CLEAR concept_alt
    END
[27D2]     SAY "Next question :"  '[voice 3, skip 1]
[27E0]     POKE [0x2744] = 0
  END
[27E4]   BLOCK (exit -> @288B)
[27E8]     AWAIT gameflag_252A
[27E9]     GUARD active_actor == Cyberquizz.talk (related 40)
[27EE]     ENDIF
[27EF]     SAY "Who said : "A murffalo belongs to whoever eats it" ? word_65535 eviscerator beauregard tromp_la_mort otto_von_smile"  '[voice 7]
[2819]     IF-BLOCK (exit -> @284F)
[281C]       GUARD concept == "eviscerator"
[281F]       ENDIF
[2820]       SAY "Mmm . can't beat a juicy murffalo steak ... Good answer ..."  '[voice 5, skip 3]
[2840]       Bof += 1
[2847]       quest += 1
[284E]       CLEAR concept_alt
    END
[284F]     IF-BLOCK (exit -> @2879)
[2852]       GUARD NOT concept == "eviscerator"
[2856]       ENDIF
[2857]       SAY "Double dummy ! Try thinking before you answer ..."  '[voice 6, skip 2]
[2871]       quest += 1
[2878]       CLEAR concept_alt
    END
[2879]     SAY "Next question :"  '[voice 3, skip 1]
[2887]     POKE [0x27E5] = 0
  END
[288B]   BLOCK (exit -> @2938)
[288F]     AWAIT gameflag_252A
[2890]     GUARD active_actor == Cyberquizz.talk (related 40)
[2895]     ENDIF
[2896]     SAY "Who is the legendary Jeph d'Ulikan ? word_65535 slimer izwal croolis migrax robot"  '[voice 4]
[28BA]     IF-BLOCK (exit -> @28EC)
[28BD]       GUARD concept == "migrax"
[28C0]       ENDIF
[28C1]       SAY "yes , the conqueror of the Black Larsen Hordes ..."  '[voice 3, skip 3]
[28DD]       Bof += 1
[28E4]       quest += 1
[28EB]       CLEAR concept_alt
    END
[28EC]     IF-BLOCK (exit -> @2926)
[28EF]       GUARD NOT concept == "migrax"
[28F3]       ENDIF
[28F4]       SAY "Jeph d'Ulikan ? That's a new one to me . You trying to be wise guy ?"  '[voice 2, skip 2]
[291E]       quest += 1
[2925]       CLEAR concept_alt
    END
[2926]     SAY "Next question :"  '[voice 3, skip 1]
[2934]     POKE [0x288C] = 0
  END
[2938]   BLOCK (exit -> @29D3)
[293C]     AWAIT gameflag_252A
[293D]     GUARD active_actor == Cyberquizz.talk (related 40)
[2942]     ENDIF
[2943]     SAY "Who said : "Every time a planet blows up , I throws up" ? word_65535 eviscerator emasculator lord_raptor tina_burner"  '[voice 1]
[2973]     IF-BLOCK (exit -> @299D)
[2976]       GUARD concept == "tina_burner"
[2979]       ENDIF
[297A]       SAY "Yes . A great song ..."  '[voice 2, skip 3]
[298E]       Bof += 1
[2995]       quest += 1
[299C]       CLEAR concept_alt
    END
[299D]     IF-BLOCK (exit -> @29C1)
[29A0]       GUARD NOT concept == "tina_burner"
[29A4]       ENDIF
[29A5]       SAY "Don't you have a radio ?"  '[voice 3, skip 2]
[29B9]       quest += 1
[29C0]       CLEAR concept_alt
    END
[29C1]     SAY "Next question :"  '[voice 3, skip 1]
[29CF]     POKE [0x2939] = 0
  END
[29D3]   BLOCK (exit -> @2A6E)
[29D7]     AWAIT gameflag_252A
[29D8]     GUARD active_actor == Cyberquizz.talk (related 40)
[29DD]     ENDIF
[29DE]     SAY "Who did Jeph d'Ulikan conquer ? word_65535 pagos_tagos black_larsen hom lord_krater"  '[voice 4]
[29FE]     IF-BLOCK (exit -> @2A2A)
[2A01]       GUARD concept == "black_larsen"
[2A04]       ENDIF
[2A05]       SAY "The Black Larsen Hordes , yes ..."  '[voice 5, skip 3]
[2A1B]       Bof += 1
[2A22]       quest += 1
[2A29]       CLEAR concept_alt
    END
[2A2A]     IF-BLOCK (exit -> @2A5C)
[2A2D]       GUARD NOT concept == "black_larsen"
[2A31]       ENDIF
[2A32]       SAY "No . I hate it when people don't know the right answer ..."  '[voice 6, skip 2]
[2A54]       quest += 1
[2A5B]       CLEAR concept_alt
    END
[2A5C]     SAY "Next question :"  '[voice 3, skip 1]
[2A6A]     POKE [0x29D4] = 0
  END
[2A6E]   BLOCK (exit -> @2B09)
[2A72]     AWAIT gameflag_252A
[2A73]     GUARD active_actor == Cyberquizz.talk (related 40)
[2A78]     ENDIF
[2A79]     SAY "Who said : "A gram of splatch is worth two rituals" ? word_65535 lord_krater metro_paul yolk"  '[voice 7]
[2AA3]     IF-BLOCK (exit -> @2AD1)
[2AA6]       GUARD concept == "yolk"
[2AA9]       ENDIF
[2AAA]       SAY "That Yolk . What a cool guy ..."  '[voice 8, skip 3]
[2AC2]       Bof += 1
[2AC9]       quest += 1
[2AD0]       CLEAR concept_alt
    END
[2AD1]     IF-BLOCK (exit -> @2AF7)
[2AD4]       GUARD NOT concept == "yolk"
[2AD8]       ENDIF
[2AD9]       SAY "Bone up on your classics, numbskull ..."  '[voice 9, skip 2]
[2AEF]       quest += 1
[2AF6]       CLEAR concept_alt
    END
[2AF7]     SAY "Next question :"  '[voice 3, skip 1]
[2B05]     POKE [0x2A6F] = 0
  END
[2B09]   BLOCK (exit -> @2BAC)
[2B0D]     AWAIT gameflag_252A
[2B0E]     GUARD active_actor == Cyberquizz.talk (related 40)
[2B13]     ENDIF
[2B14]     SAY "How many murffalos does it take to reproduce ? word_65535 two four six eight thirteen"  '[voice 8]
[2B3C]     IF-BLOCK (exit -> @2B72)
[2B3F]       GUARD concept == "thirteen"
[2B42]       ENDIF
[2B43]       SAY "Right . One in the middle and the others all around ..."  '[voice 7, skip 3]
[2B63]       Bof += 1
[2B6A]       quest += 1
[2B71]       CLEAR concept_alt
    END
[2B72]     IF-BLOCK (exit -> @2B9A)
[2B75]       GUARD NOT concept == "thirteen"
[2B79]       ENDIF
[2B7A]       SAY "No , that wouldn't work for murffalos ..."  '[voice 6, skip 2]
[2B92]       quest += 1
[2B99]       CLEAR concept_alt
    END
[2B9A]     SAY "Next question :"  '[voice 3, skip 1]
[2BA8]     POKE [0x2B0A] = 0
  END
[2BAC]   BLOCK (exit -> @2C3D)
[2BB0]     AWAIT gameflag_252A
[2BB1]     GUARD active_actor == Cyberquizz.talk (related 40)
[2BB6]     ENDIF
[2BB7]     SAY "Name a planet of the Gran Arber? word_65535 qx20 tumul vista ebony mastachok"  '[voice 5]
[2BDB]     IF-BLOCK (exit -> @2C03)
[2BDE]       GUARD concept == "vista"
[2BE1]       ENDIF
[2BE2]       SAY "Pretty sharp on geography ."  '[voice 4, skip 3]
[2BF4]       Bof += 1
[2BFB]       quest += 1
[2C02]       CLEAR concept_alt
    END
[2C03]     IF-BLOCK (exit -> @2C2B)
[2C06]       GUARD NOT concept == "vista"
[2C0A]       ENDIF
[2C0B]       SAY "Geography's not exactly your specialisation , huh ?"  '[voice 3, skip 2]
[2C23]       quest += 1
[2C2A]       CLEAR concept_alt
    END
[2C2B]     SAY "Next question :"  '[voice 3, skip 1]
[2C39]     POKE [0x2BAD] = 0
  END
[2C3D]   BLOCK (exit -> @2CD0)
[2C41]     AWAIT gameflag_252A
[2C42]     GUARD active_actor == Cyberquizz.talk (related 40)
[2C47]     ENDIF
[2C48]     SAY "Who invented Buggol democracy ? word_65535 strum Bob_Morlock cyborg_1st exxos"  '[voice 2]
[2C66]     IF-BLOCK (exit -> @2C94)
[2C69]       GUARD concept == "cyborg_1st"
[2C6C]       ENDIF
[2C6D]       SAY "Say , you're pretty hot on history ..."  '[voice 1, skip 3]
[2C85]       Bof += 1
[2C8C]       quest += 1
[2C93]       CLEAR concept_alt
    END
[2C94]     IF-BLOCK (exit -> @2CBE)
[2C97]       GUARD NOT concept == "cyborg_1st"
[2C9B]       ENDIF
[2C9C]       SAY "History never was your strong point , huh ?"  '[voice 0, skip 2]
[2CB6]       quest += 1
[2CBD]       CLEAR concept_alt
    END
[2CBE]     SAY "Next question :"  '[voice 3, skip 1]
[2CCC]     POKE [0x2C3E] = 0
  END
[2CD0]   BLOCK (exit -> @2D65)
[2CD4]     AWAIT gameflag_252A
[2CD5]     GUARD active_actor == Cyberquizz.talk (related 40)
[2CDA]     ENDIF
[2CDB]     SAY "What do Tubular Brains eat ? word_65535 klakos plastok radium hay murffalos"  '[voice 1]
[2CFD]     IF-BLOCK (exit -> @2D21)
[2D00]       GUARD concept == "plastok"
[2D03]       ENDIF
[2D04]       SAY "Protein enriched plastok."  '[voice 2, skip 3]
[2D12]       Bof += 1
[2D19]       quest += 1
[2D20]       CLEAR concept_alt
    END
[2D21]     IF-BLOCK (exit -> @2D53)
[2D24]       GUARD NOT concept == "plastok"
[2D28]       ENDIF
[2D29]       SAY "Just don't bother taking a job as a cook for Tubular Brains ."  '[voice 3, skip 2]
[2D4B]       quest += 1
[2D52]       CLEAR concept_alt
    END
[2D53]     SAY "Next question :"  '[voice 3, skip 1]
[2D61]     POKE [0x2CD1] = 0
  END
[2D65]   BLOCK (exit -> @2DFA)
[2D69]     AWAIT gameflag_252A
[2D6A]     GUARD active_actor == Cyberquizz.talk (related 40)
[2D6F]     ENDIF
[2D70]     SAY "Name a planet with rings ... word_65535 erazor rondo bonus pterra"  '[voice 4]
[2D90]     IF-BLOCK (exit -> @2DBE)
[2D93]       GUARD concept == "bonus"
[2D96]       ENDIF
[2D97]       SAY "Bonus has the universe's most beautiful rings ."  '[voice 5, skip 3]
[2DAF]       Bof += 1
[2DB6]       quest += 1
[2DBD]       CLEAR concept_alt
    END
[2DBE]     IF-BLOCK (exit -> @2DE8)
[2DC1]       GUARD NOT concept == "bonus"
[2DC5]       ENDIF
[2DC6]       SAY "Rings ... You know , those round things ."  '[voice 6, skip 2]
[2DE0]       quest += 1
[2DE7]       CLEAR concept_alt
    END
[2DE8]     SAY "Next question :"  '[voice 3, skip 1]
[2DF6]     POKE [0x2D66] = 0
  END
[2DFA]   BLOCK (exit -> @2E93)
[2DFE]     AWAIT gameflag_252A
[2DFF]     GUARD active_actor == Cyberquizz.talk (related 40)
[2E04]     ENDIF
[2E05]     SAY "How do Scorps reproduce ? word_65535 vivisection parthenogenesis mimicry chance"  '[voice 7]
[2E23]     IF-BLOCK (exit -> @2E57)
[2E26]       GUARD concept == "parthenogenesis"
[2E29]       ENDIF
[2E2A]       SAY "Yes , you chop 'em up up and they regrow ."  '[voice 8, skip 3]
[2E48]       Bof += 1
[2E4F]       quest += 1
[2E56]       CLEAR concept_alt
    END
[2E57]     IF-BLOCK (exit -> @2E81)
[2E5A]       GUARD NOT concept == "parthenogenesis"
[2E5E]       ENDIF
[2E5F]       SAY "Maybe you should splash out on a dictionary ."  '[voice 9, skip 2]
[2E79]       quest += 1
[2E80]       CLEAR concept_alt
    END
[2E81]     SAY "Next question :"  '[voice 3, skip 1]
[2E8F]     POKE [0x2DFB] = 0
  END
[2E93]   BLOCK (exit -> @2F2A)
[2E97]     AWAIT gameflag_252A
[2E98]     GUARD active_actor == Cyberquizz.talk (related 40)
[2E9D]     ENDIF
[2E9E]     SAY "Name a powerful highly concentrated explosive ... word_65535 big_bang splach plastok fuzz"  '[voice 10]
[2EC0]     IF-BLOCK (exit -> @2EF4)
[2EC3]       GUARD concept == "splach"
[2EC6]       ENDIF
[2EC7]       SAY "Well done . Splatch is composed of oxydised Larsen liver ..."  '[voice 1, skip 3]
[2EE5]       Bof += 1
[2EEC]       quest += 1
[2EF3]       CLEAR concept_alt
    END
[2EF4]     IF-BLOCK (exit -> @2F18)
[2EF7]       GUARD NOT concept == "splach"
[2EFB]       ENDIF
[2EFC]       SAY "You sure blew that one !"  '[voice 2, skip 2]
[2F10]       quest += 1
[2F17]       CLEAR concept_alt
    END
[2F18]     SAY "Next question :"  '[voice 3, skip 1]
[2F26]     POKE [0x2E94] = 0
  END
[2F2A]   BLOCK (exit -> @2FD3)
[2F2E]     AWAIT gameflag_252A
[2F2F]     GUARD active_actor == Cyberquizz.talk (related 40)
[2F34]     ENDIF
[2F35]     SAY "What is the name of the queen of the PATAGOS? word_65535 tina_burner scorpia pistilla umatika"  '[voice 3]
[2F5D]     IF-BLOCK (exit -> @2F95)
[2F60]       GUARD concept == "umatika"
[2F63]       ENDIF
[2F64]       SAY "Scorpia , daughter to king Betakam fourth , of the eighteenth dynasty ..."  '[voice 4, skip 3]
[2F86]       Bof += 1
[2F8D]       quest += 1
[2F94]       CLEAR concept_alt
    END
[2F95]     IF-BLOCK (exit -> @2FC1)
[2F98]       GUARD NOT concept == "umatika"
[2F9C]       ENDIF
[2F9D]       SAY "How could you not know the answer to that ?"  '[voice 5, skip 2]
[2FB9]       quest += 1
[2FC0]       CLEAR concept_alt
    END
[2FC1]     SAY "Next question :"  '[voice 3, skip 1]
[2FCF]     POKE [0x2F2B] = 0
  END
[2FD3]   BLOCK (exit -> @306C)
[2FD7]     AWAIT gameflag_252A
[2FD8]     GUARD active_actor == Cyberquizz.talk (related 40)
[2FDD]     ENDIF
[2FDE]     SAY "Who said Trump tails came straight from their backsides ? word_65535 lord_segelaxx inquisitor slim_gelati tromp_deustache"  '[voice 1]
[3006]     IF-BLOCK (exit -> @3038)
[3009]       GUARD concept == "lord_segelaxx"
[300C]       ENDIF
[300D]       SAY "Smoking Trump tails is very bad for your health ."  '[voice 2, skip 3]
[3029]       Bof += 1
[3030]       quest += 1
[3037]       CLEAR concept_alt
    END
[3038]     IF-BLOCK (exit -> @305A)
[303B]       GUARD NOT concept == "lord_segelaxx"
[303F]       ENDIF
[3040]       SAY "Not an easy question ..."  '[voice 3, skip 2]
[3052]       quest += 1
[3059]       CLEAR concept_alt
    END
[305A]     SAY "Next question :"  '[voice 3, skip 1]
[3068]     POKE [0x2FD4] = 0
  END
[306C]   BLOCK (exit -> @30FD)
[3070]     AWAIT gameflag_252A
[3071]     GUARD active_actor == Cyberquizz.talk (related 40)
[3076]     ENDIF
[3077]     SAY "In what city is sex sold illicitly ? word_65535 los_demonios venusia attroxcity trashtown"  '[voice 4]
[309B]     IF-BLOCK (exit -> @30C9)
[309E]       GUARD concept == "attroxcity"
[30A1]       ENDIF
[30A2]       SAY "Yes . Attroxcity, the city of Slimers !"  '[voice 5, skip 3]
[30BA]       Bof += 1
[30C1]       quest += 1
[30C8]       CLEAR concept_alt
    END
[30C9]     IF-BLOCK (exit -> @30ED)
[30CC]       GUARD NOT concept == "attroxcity"
[30D0]       ENDIF
[30D1]       SAY "That answer was so poor ..."  '[voice 6, skip 2]
[30E5]       quest += 1
[30EC]       CLEAR concept_alt
    END
[30ED]     SAY "Next question:"  '[voice 3, skip 1]
[30F9]     POKE [0x306D] = 0
  END
[30FD]   BLOCK (exit -> @31A6)
[3101]     AWAIT gameflag_252A
[3102]     GUARD active_actor == Cyberquizz.talk (related 40)
[3107]     ENDIF
[3108]     SAY "Who painted the famous portrait of the Great Yolk ? word_65535 van_ish van_gelis van_et van_deta van_tage van_gogue"  '[voice 7]
[3134]     IF-BLOCK (exit -> @316A)
[3137]       GUARD concept == "van_gogue"
[313A]       ENDIF
[313B]       SAY "Van Gogue painted it with the Great Yolk's own yellow blood ..."  '[voice 8, skip 3]
[315B]       Bof += 1
[3162]       quest += 1
[3169]       CLEAR concept_alt
    END
[316A]     IF-BLOCK (exit -> @3194)
[316D]       GUARD NOT concept == "van_gogue"
[3171]       ENDIF
[3172]       SAY "That was supposed to be an easy one ..."  '[voice 9, skip 2]
[318C]       quest += 1
[3193]       CLEAR concept_alt
    END
[3194]     SAY "Next question :"  '[voice 3, skip 1]
[31A2]     POKE [0x30FE] = 0
  END
[31A6]   BLOCK (exit -> @323D)
[31AA]     AWAIT gameflag_252A
[31AB]     GUARD active_actor == Cyberquizz.talk (related 40)
[31B0]     ENDIF
[31B1]     SAY "Who wrote the famous book called the Croolicum ? word_65535 lord_raptor lord_ship von_spacecraft nobody"  '[voice 10]
[31D7]     IF-BLOCK (exit -> @3203)
[31DA]       GUARD concept == "nobody"
[31DD]       ENDIF
[31DE]       SAY "You avoided the trap most skilfully !"  '[voice 0, skip 3]
[31F4]       Bof += 1
[31FB]       quest += 1
[3202]       CLEAR concept_alt
    END
[3203]     IF-BLOCK (exit -> @322B)
[3206]       GUARD NOT concept == "nobody"
[320A]       ENDIF
[320B]       SAY "The Croolicum ? You can't be serious !"  '[voice 1, skip 2]
[3223]       quest += 1
[322A]       CLEAR concept_alt
    END
[322B]     SAY "Next question :"  '[voice 3, skip 1]
[3239]     POKE [0x31A7] = 0
  END
[323D]   BLOCK (exit -> @32D0)
[3241]     AWAIT gameflag_252A
[3242]     GUARD active_actor == Cyberquizz.talk (related 40)
[3247]     ENDIF
[3248]     SAY "Who was the universe's first trump tail smoker ? word_65535 al_hure sebasto_paul lord_sirtaki jeph_dulikan"  '[voice 2]
[326E]     IF-BLOCK (exit -> @329E)
[3271]       GUARD concept == "al_hure"
[3274]       ENDIF
[3275]       SAY "That was an answer to be proud of ..."  '[voice 2, skip 3]
[328F]       Bof += 1
[3296]       quest += 1
[329D]       CLEAR concept_alt
    END
[329E]     IF-BLOCK (exit -> @32BE)
[32A1]       GUARD NOT concept == "al_hure"
[32A5]       ENDIF
[32A6]       SAY "Bad mistake there ..."  '[voice 3, skip 2]
[32B6]       quest += 1
[32BD]       CLEAR concept_alt
    END
[32BE]     SAY "Next question :"  '[voice 3, skip 1]
[32CC]     POKE [0x323E] = 0
  END
[32D0]   BLOCK (exit -> @3371)
[32D4]     AWAIT gameflag_252A
[32D5]     GUARD active_actor == Cyberquizz.talk (related 40)
[32DA]     ENDIF
[32DB]     SAY "Who said : "Silence feeds on thunder and fury" ? word_65535 yolk joy_stica hom super_zen"  '[voice 4]
[3303]     IF-BLOCK (exit -> @3331)
[3306]       GUARD concept == "yolk"
[3309]       ENDIF
[330A]       SAY "Yes . May his soul blow bubbles ..."  '[voice 4, skip 3]
[3322]       Bof += 1
[3329]       quest += 1
[3330]       CLEAR concept_alt
    END
[3331]     IF-BLOCK (exit -> @335F)
[3334]       GUARD NOT concept == "yolk"
[3338]       ENDIF
[3339]       SAY "You really ought to take an interest in the classics ..."  '[voice 5, skip 2]
[3357]       quest += 1
[335E]       CLEAR concept_alt
    END
[335F]     SAY "Next question :"  '[voice 3, skip 1]
[336D]     POKE [0x32D1] = 0
  END
[3371]   BLOCK (exit -> @33F2)
[3375]     AWAIT gameflag_252A
[3376]     GUARD active_actor == Cyberquizz.talk (related 40)
[337B]     ENDIF
[337C]     SAY "Name one hit by the Migrators ? word_65535 crush_me_baby space_maker hello_dolly love_me_do"  '[voice 6]
[339E]     IF-BLOCK (exit -> @33C4)
[33A1]       GUARD concept == "crush_me_baby"
[33A4]       ENDIF
[33A5]       SAY "Cruuuush meee baaaayybeeee !"  '[voice 7, skip 3]
[33B5]       Bof += 1
[33BC]       quest += 1
[33C3]       CLEAR concept_alt
    END
[33C4]     IF-BLOCK (exit -> @33E0)
[33C7]       GUARD NOT concept == "crush_me_baby.."
[33CB]       ENDIF
[33CC]       SAY "Ouch ..."  '[voice 8, skip 2]
[33D8]       quest += 1
[33DF]       CLEAR concept_alt
    END
[33E0]     SAY "Next question :"  '[voice 3, skip 1]
[33EE]     POKE [0x3372] = 0
  END
[33F2]   BLOCK (exit -> @347F)
[33F6]     AWAIT gameflag_252A
[33F7]     GUARD active_actor == Cyberquizz.talk (related 40)
[33FC]     ENDIF
[33FD]     SAY "Who is the oldest living being in the universe ? word_65535 otto_von_smile Bob_Morlock super_tromp techno_paul"  '[voice 8]
[3425]     IF-BLOCK (exit -> @344B)
[3428]       GUARD concept == "Bob_Morlock"
[342B]       ENDIF
[342C]       SAY "Good ol' Bob !"  '[voice 8, skip 3]
[343C]       Bof += 1
[3443]       quest += 1
[344A]       CLEAR concept_alt
    END
[344B]     IF-BLOCK (exit -> @346D)
[344E]       GUARD NOT concept == "Bob_Morlock"
[3452]       ENDIF
[3453]       SAY "I said the oldest ..."  '[voice 6, skip 2]
[3465]       quest += 1
[346C]       CLEAR concept_alt
    END
[346D]     SAY "Next question :"  '[voice 3, skip 1]
[347B]     POKE [0x33F3] = 0
  END
[347F]   BLOCK (exit -> @351C)
[3483]     AWAIT gameflag_252A
[3484]     GUARD active_actor == Cyberquizz.talk (related 40)
[3489]     ENDIF
[348A]     POKE [0x21D4] = 0
[348E]     SAY "Which famous dancer is the star of "Hold me in your tentacles" ? word_65535 torka tina_burner cybertha joy_stika"  '[voice 8]
[34BC]     IF-BLOCK (exit -> @34E2)
[34BF]       GUARD concept == "tina_burner"
[34C2]       ENDIF
[34C3]       SAY "What a star !"  '[voice 6, skip 3]
[34D3]       Bof += 1
[34DA]       quest += 1
[34E1]       CLEAR concept_alt
    END
[34E2]     IF-BLOCK (exit -> @350A)
[34E5]       GUARD NOT concept == "tina_burner"
[34E9]       ENDIF
[34EA]       SAY "Come on !! You're such a nitwit ..."  '[voice 9, skip 2]
[3502]       quest += 1
[3509]       CLEAR concept_alt
    END
[350A]     SAY "Next question :"  '[voice 3, skip 1]
[3518]     POKE [0x3480] = 0
  END
[351C]   BLOCK (exit -> @35B1)
[3520]     AWAIT gameflag_252A
[3521]     GUARD active_actor == Cyberquizz.talk (related 40)
[3526]     ENDIF
[3527]     SAY "What do Migrax prefer to drink ? word_65535 processed_liver frozen_migrata fermented_egg heavy_water"  '[voice 6]
[3549]     IF-BLOCK (exit -> @3579)
[354C]       GUARD concept == "processed_liver"
[354F]       ENDIF
[3550]       SAY "Well , that was a little too easy ..."  '[voice 9, skip 3]
[356A]       Bof += 1
[3571]       quest += 1
[3578]       CLEAR concept_alt
    END
[3579]     IF-BLOCK (exit -> @359F)
[357C]       GUARD NOT concept == "processed_liver"
[3580]       ENDIF
[3581]       SAY "You can't have met many Migrax ."  '[voice 3, skip 2]
[3597]       quest += 1
[359E]       CLEAR concept_alt
    END
[359F]     SAY "Next question :"  '[voice 3, skip 1]
[35AD]     POKE [0x351D] = 0
  END
[35B1]   BLOCK (exit -> @3654)
[35B5]     AWAIT gameflag_252A
[35B6]     GUARD active_actor == Cyberquizz.talk (related 40)
[35BB]     ENDIF
[35BC]     SAY "Who said :"ondoyant pretty , Croolis drool . ondoyant ugly , Croolis get cruel ." word_65535 nobody maxxon yolk"  '[voice 6]
[35EC]     IF-BLOCK (exit -> @361C)
[35EF]       GUARD concept == "maxxon"
[35F2]       ENDIF
[35F3]       SAY "Ah , Maxxon . Such a poetic soul ..."  '[voice 0, skip 3]
[360D]       Bof += 1
[3614]       quest += 1
[361B]       CLEAR concept_alt
    END
[361C]     IF-BLOCK (exit -> @3642)
[361F]       GUARD NOT concept == "maxxon"
[3623]       ENDIF
[3624]       SAY "You have no feeling for poetry ..."  '[voice 3, skip 2]
[363A]       quest += 1
[3641]       CLEAR concept_alt
    END
[3642]     SAY "Next question :"  '[voice 3, skip 1]
[3650]     POKE [0x35B2] = 0
  END
[3654]   BLOCK (exit -> @36ED)
[3658]     AWAIT gameflag_252A
[3659]     GUARD active_actor == Cyberquizz.talk (related 40)
[365E]     ENDIF
[365F]     SAY "The tomb of the Great Yolk is on which planet ? word_65535 rondo ekatomb vista magnus"  '[voice 0]
[3689]     IF-BLOCK (exit -> @36B5)
[368C]       GUARD concept == "vista"
[368F]       ENDIF
[3690]       SAY "In the S.C.R.U.T. palace on Vista ."  '[voice 6, skip 3]
[36A6]       Bof += 1
[36AD]       quest += 1
[36B4]       CLEAR concept_alt
    END
[36B5]     IF-BLOCK (exit -> @36DB)
[36B8]       GUARD NOT concept == "vista"
[36BC]       ENDIF
[36BD]       SAY "You don't know the S.C.R.U.T. palace ?"  '[voice 9, skip 2]
[36D3]       quest += 1
[36DA]       CLEAR concept_alt
    END
[36DB]     SAY "Next question :"  '[voice 3, skip 1]
[36E9]     POKE [0x3655] = 0
  END
[36ED]   BLOCK (exit -> @3786)
[36F1]     AWAIT gameflag_252A
[36F2]     GUARD active_actor == Cyberquizz.talk (related 40)
[36F7]     ENDIF
[36F8]     SAY "On which planet can you hear voices ? word_65535 pterra cyberock kult tumul"  '[voice 7]
[371C]     IF-BLOCK (exit -> @374E)
[371F]       GUARD concept == "kult"
[3722]       ENDIF
[3723]       SAY "Yes , the voice which foretells the future , Commander..."  '[voice 3, skip 3]
[373F]       Bof += 1
[3746]       quest += 1
[374D]       CLEAR concept_alt
    END
[374E]     IF-BLOCK (exit -> @3774)
[3751]       GUARD NOT concept == "kult"
[3755]       ENDIF
[3756]       SAY "You just have to be deaf ..."  '[voice 0, skip 2]
[376C]       quest += 1
[3773]       CLEAR concept_alt
    END
[3774]     SAY "Next question :"  '[voice 3, skip 1]
[3782]     POKE [0x36EE] = 0
  END
[3786]   BLOCK (exit -> @3817)
[378A]     AWAIT gameflag_252A
[378B]     GUARD active_actor == Cyberquizz.talk (related 40)
[3790]     ENDIF
[3791]     SAY "Which sun shines on the planet Tumul ? word_65535 ex897 gladis corpo negratom"  '[voice 1]
[37B5]     IF-BLOCK (exit -> @37DF)
[37B8]       GUARD concept == "gladis"
[37BB]       ENDIF
[37BC]       SAY "Gladis , the smart star ."  '[voice 3, skip 3]
[37D0]       Bof += 1
[37D7]       quest += 1
[37DE]       CLEAR concept_alt
    END
[37DF]     IF-BLOCK (exit -> @3805)
[37E2]       GUARD NOT concept == "gladis"
[37E6]       ENDIF
[37E7]       SAY "Havn't you ever been to Tumul ?"  '[voice 1, skip 2]
[37FD]       quest += 1
[3804]       CLEAR concept_alt
    END
[3805]     SAY "Next question :"  '[voice 3, skip 1]
[3813]     POKE [0x3787] = 0
  END
[3817]   BLOCK (exit -> @38B8)
[381B]     AWAIT gameflag_252A
[381C]     GUARD active_actor == Cyberquizz.talk (related 40)
[3821]     ENDIF
[3822]     SAY "How many moons does the planet Moskito have ? word_65535 five twelve zero thirty_two"  '[voice 10]
[3848]     IF-BLOCK (exit -> @3876)
[384B]       GUARD concept == "zero"
[384E]       ENDIF
[384F]       SAY "Good answer . Moskito has no moons ."  '[voice 1, skip 3]
[3867]       Bof += 1
[386E]       quest += 1
[3875]       CLEAR concept_alt
    END
[3876]     IF-BLOCK (exit -> @38A0)
[3879]       GUARD NOT concept == "zero"
[387D]       ENDIF
[387E]       SAY "Maybe you shouldn't trust your intuition so much ..."  '[voice 3, skip 2]
[3898]       quest += 1
[389F]       CLEAR concept_alt
    END
[38A0]     SAY "That was the final question ..."  '[voice 3, skip 1]
[38B4]     POKE [0x3818] = 0
  END
[38B8]   BLOCK (exit -> @3943)
[38BC]     AWAIT gameflag_252A
[38BD]     GUARD active_actor == Emasculator.talk (related 40)
[38C2]     GUARD J1 == 0
[38C9]     ENDIF
[38CA]     SAY "What you want , stranger ? You want murffalo meat ? Me be new sales assistant ..."  '[voice 1]
[38F4]     SAY "You have CRED ?"  '[voice 2]
[3904]     SAY "No CRED , no murfallo meat ..."  '[voice 4]
[391A]     SAY "Me much work ... You not waste my time ..."  '[voice 4]
[3936]     SAY "stop"  '[skip 1]
[3940]     END PRESENTATION Emasculator.talk
  END
[3943]   BLOCK (exit -> @39C8)
[3947]     AWAIT gameflag_252A
[3948]     GUARD rec_1088 == 3224
[394D]     GUARD rec_06DA == 3254
[3952]     GUARD active_actor == Amigo.talk (related 40)
[3957]     ENDIF
[3958]     IF-BLOCK (exit -> @39A7)
[395B]       GUARD rec_06F8 == 1
[3962]       ENDIF
[3963]       SAY "Hic! Spare a cred , handsome ? hic !!! Life's so hard ... Hic ..."  '[voice 1]
[3989]       SAY "Just a teensy weensy cred ... For a drink ... Hic!"  '[voice 2]
    END
[39A7]     SAY "Life's so hic hard ..."
[39B9]     SAY "..."  '[skip 1]
[39C3]     OP_C1 C1 D0 13 CE 0C
  END
[39C8]   BLOCK (exit -> @3C4F)
[39CC]     AWAIT gameflag_252A
[39CD]     GUARD rec_1088 == 3224
[39D2]     GUARD E1 == 0
[39D9]     GUARD active_actor == Tina_Burner.talk (related 40)
[39DE]     ENDIF
[39DF]     SAY "Hey ! Handsome stranger . You come to see Tina ? ..."  '[voice 1]
[39FF]     SAY "You're in pretty bad shape . Burny'll fix you up .."  '[voice 1]
[3A1D]     SAY "You're my idea of a real man ... You got something special ... I can sniff out real males from lights years away ..."  '[voice 2]
[3A55]     SAY "Honk reporting at this time :"
[3A69]     SAY "Be on your guard , Commander . Resist her seductive charms ..."
[3A89]     SAY "I'm Tina Burner , but my best friends call me Burny ..."  '[voice 3]
[3AA9]     SAY "Commander , your pulse rate is way too high ... I'll just give you a shot of something ... Commander , are you listening ?"
[3AE3]     SAY "Hey! handsome! Still living with mommy and daddy ? word_65535 yes no"  '[voice 4]
[3B05]     IF-BLOCK (exit -> @3B37)
[3B08]       GUARD concept == "yes"
[3B0B]       ENDIF
[3B0C]       SAY "AAAARHHH ! He still needs his mommy ... Ha ! Ha ! Hee ! Hee ! ..."  '[voice 5]
[3B36]       CLEAR concept_alt
    END
[3B37]     IF-BLOCK (exit -> @3B6F)
[3B3A]       GUARD concept == "no"
[3B3D]       ENDIF
[3B3E]       SAY "Say ! You're blushing ... Is he a shy little man ? Ha ! Ha ! Hee ! Hee !"  '[voice 6]
[3B6E]       CLEAR concept_alt
    END
[3B6F]     SAY "You know I'm a star , right ? On the planet Eden , all the males are crazy about me ..."  '[voice 7]
[3BA1]     SAY "I sing every night here at the PURPLE HAZE . The universe's hottest nite spot ..."  '[voice 8]
[3BC9]     SAY "Why don't you just tell me all about yourself ... Honeypoo ..."  '[voice 4]
[3BE9]     SAY "Commander , you're gonna have to cool down ... Commander !"
[3C07]     SAY "Going so soon ? Ohhh! Come back real soon ... My little heart just races when you're near ... word_65535 bye_bye"  '[voice 6]
[3C39]     SAY "Bye bye..."  '[voice 4, skip 2]
[3C45]     E1 = 1
[3C4C]     END PRESENTATION Tina_Burner.talk
  END
[3C4F]   BLOCK (exit -> @3E3A)
[3C53]     AWAIT gameflag_252A
[3C54]     GUARD rec_1088 == 3224
[3C59]     GUARD E1 == 1
[3C60]     GUARD active_actor == Tina_Burner.talk (related 40)
[3C65]     ENDIF
[3C66]     SAY "Aaaahhh! It's my warrior hero ... Back to see me ... I knew you'd come ..."  '[voice 2]
[3C8E]     SAY "Honk here:"
[3C9A]     SAY "Oh no ... She's doing it again ..."
[3CB2]     SAY "You missed your little Burnypoo ... How 'bout a drink , muscleman ?"  '[voice 3]
[3CD4]     IF-BLOCK (exit -> @3D64)
[3CD7]       GUARD rec_1392 == 40
[3CDC]       ENDIF
[3CDD]       SAY "Commander ... Why don't we give her Kran_Dobu's guitar ?"
[3CF9]       SAY "It might calm her down ..."
[3D0D]       SAY "TELEPORT GUITAR TO PLANET EDEN word_65535 teleport refuse"
[3D27]       IF-BLOCK (exit -> @3D48)
[3D2A]         GUARD concept == "teleport"
[3D2D]         ENDIF
[3D2E]         SAY "TELEPORTING GUITAR TO TINA BURNER"  '[skip 2]
[3D40]         OP_CD CD 30 00 7E 13 72 08
[3D47]         CLEAR concept_alt
      END
[3D48]       IF-BLOCK (exit -> @3D64)
[3D4B]         GUARD concept == "refuse"
[3D4E]         ENDIF
[3D4F]         SAY "Whatever you say , Commander ..."  '[skip 1]
[3D63]         CLEAR concept_alt
      END
    END
[3D64]     IF-BLOCK (exit -> @3DE9)
[3D67]       GUARD rec_1392 == 2162
[3D6C]       ENDIF
[3D6D]       SAY "Oh , a guitar ! . For little me ? ... Hee ! Hee ! Hee !"  '[voice 1]
[3D97]       SAY "You're just adorable ... Hee ! Hee!"  '[voice 2]
[3DAD]       SAY "I just have to go play ..."  '[voice 5]
[3DC3]       SAY "Goodbye , gorgeous one ..."  '[voice 8]
[3DD5]       SAY "..."  '[skip 2]
[3DDF]       E1 = 2
[3DE6]       END PRESENTATION Tina_Burner.talk
    END
[3DE9]     SAY "Tell me about yourself ..."  '[voice 5]
[3DFB]     SAY "Leaving already ? Ohhh! Come back soon ... My little heart flutters when you're close to me ... word_65535 bye_bye"  '[voice 6]
[3E2B]     SAY "Bye bye..."  '[voice 4, skip 1]
[3E37]     END PRESENTATION Tina_Burner.talk
  END
[3E3A]   BLOCK (exit -> @4040)
[3E3E]     AWAIT gameflag_252A
[3E3F]     GUARD rec_1088 == 3224
[3E44]     GUARD E1 == 2
[3E4B]     GUARD active_actor == Tina_Burner.talk (related 40)
[3E50]     ENDIF
[3E51]     SAY "Aaaahhh! It's my manly warrior ... You came back to me ..."  '[voice 2]
[3E71]     IF-BLOCK (exit -> @4003)
[3E74]       GUARD rec_0548 > 0
[3E7B]       ENDIF
[3E7C]       SAY "Take me with you , stranger ... This place is dangerous for me ."  '[voice 3]
[3EA0]       SAY "I can't explain it but this place has bad vibes for me ... Take me with you ..."  '[voice 5]
[3ECC]       SAY "No ! Don't do it , Commander ... It wouldn't be good for the Ark ... Commander ! Cap'n Bob's gonna be mad at you ..."
[3F08]       SAY "TELEPORT TINA BURNER TO CRYOBOX: word_65535 TELEPORT REFUSE"
[3F22]       IF-BLOCK (exit -> @3F44)
[3F25]         GUARD concept == "TELEPORT"
[3F28]         ENDIF
[3F29]         SAY "TINA BURNER TELEPORTED TO CRYOBOX"  '[skip 3]
[3F3B]         rec_088A = 65535
[3F40]         CLEAR concept_alt
[3F41]         END PRESENTATION Tina_Burner.talk
      END
[3F44]       IF-BLOCK (exit -> @4003)
[3F47]         GUARD concept == "REFUSE"
[3F4A]         ENDIF
[3F4B]         SAY "Smart thinking , Commander !!!"
[3F5D]         SAY "What !!! You refuse to take me with you !!! You dirty son of a Croolas ..."  '[voice 3]
[3F87]         SAY "Get out of here !!! CRY CRY"  '[voice 2]
[3F9D]         SAY "CRY ... SNIFF ... CRY CRY ... HOT TEARS ..."  '[voice 6]
[3FB9]         SAY "SNIFF ... CRY CRY ... HOT TEARS ..."  '[voice 6]
[3FD1]         SAY "Maybe we should have said yes , Commander ... She's taking it badly ..."
[3FF5]         SAY "..."  '[skip 2]
[3FFF]         CLEAR concept_alt
[4000]         END PRESENTATION Tina_Burner.talk
      END
    END
[4003]     SAY "Sorry , but the bar's closed ..."  '[voice 5]
[4019]     SAY "You can't stay here ..."  '[voice 5]
[402B]     SAY "Bye bye , handsome ..."  '[voice 6, skip 1]
[403D]     END PRESENTATION Tina_Burner.talk
  END
[4040]   BLOCK (exit -> @41C2)
[4044]     AWAIT gameflag_274F
[4045]     GUARD active_actor == Tina_Burner.talk (related 40)
[404A]     ENDIF
[404B]     SAY "Hey ! Love the ship , Commander ! And who's the senior citizen snoring beside me ???"  '[voice 3]
[4075]     SAY "You know , your onboard Computer can't keep his scanner off me ..."  '[voice 2]
[4097]     SAY "What !!! She's just crazy , Commander . I wouldn't scan her in a million years !!!"
[40C1]     SAY "Commander , I believe you know a charming musician on the planet MOSKITO ..."  '[voice 5]
[40E5]     SAY "Your domestic robot told me ... The one who does the cooking ..."  '[voice 3]
[4107]     SAY "Domestic robot , huh ! You droopy bag !"
[4121]     SAY "Watch your language , tincan !"  '[voice 2]
[4135]     SAY "Ahhh !!! At least I'm not ugly and I don't stink like you !"
[4159]     SAY "Hey !!! Who gave you the right to insult me ? ... I'm gonna have a breakdown , Commander"  '[voice 8]
[4187]     SAY "CRY CRY ..."  '[voice 5]
[4195]     SAY "Commander , she really tires me out ..."
[41AD]     SAY "..."  '[skip 3]
[41B7]     POKE [0x4041] = 0
[41BB]     POKE [0x41C3] = 1
[41BF]     END PRESENTATION Tina_Burner.talk
  END
[41C2]   GOTO @423C
[41C6]   AWAIT gameflag_274F
[41C7]   GUARD active_actor == Tina_Burner.talk (related 40)
[41CC]   ENDIF
[41CD]   SAY "CRY CRY ..."  '[voice 1]
[41DB]   SAY "Commander , I refuse to stay here with that metal sex maniac ..."  '[voice 2]
[41FD]   SAY "Drop me off on a planet someplace ... Where that musician guy is ..."  '[voice 3]
[4221]   SAY "CRY CRY ..."  '[voice 5]
[422F]   SAY "..."  '[skip 1]
[4239]   END PRESENTATION Tina_Burner.talk
[423C]   BLOCK (exit -> @44B0)
[4240]     AWAIT gameflag_252A
[4241]     GUARD K1 == 0
[4248]     GUARD active_actor == Migrator.talk (related 40)
[424D]     ENDIF
[424E]     SAY "Mmm ! What strange thing ... Who you be ?"  '[voice 1]
[426A]     SAY "What your name , stranger ? word_65535 princy ziggie michael commander_blood jimmy billy"  '[voice 1]
[428E]     IF-BLOCK (exit -> @42C8)
[4291]       GUARD concept == "commander_blood"
[4294]       ENDIF
[4295]       SAY "Me greet you , Commander ."  '[voice 2]
[42A9]       SAY "Me hear about you . You be friend of Bronko ..."  '[voice 3, skip 1]
[42C7]       CLEAR concept_alt
    END
[42C8]     IF-BLOCK (exit -> @42FE)
[42CB]       GUARD concept == "princy"
[42CE]       ENDIF
[42CF]       SAY "Me greet you , friend Princy"  '[voice 3]
[42E3]       SAY "You be Princy ... You not small ??? ..."  '[voice 5, skip 1]
[42FD]       CLEAR concept_alt
    END
[42FE]     IF-BLOCK (exit -> @4338)
[4301]       GUARD concept == "ziggie"
[4304]       ENDIF
[4305]       SAY "Me greet you , friend Ziggie"  '[voice 2]
[4319]       SAY "You have nice makeup today . Me not recognize you ..."  '[voice 3, skip 1]
[4337]       CLEAR concept_alt
    END
[4338]     IF-BLOCK (exit -> @436C)
[433B]       GUARD concept == "michael"
[433E]       ENDIF
[433F]       SAY "Me greet you , friend Michael"  '[voice 3]
[4353]       SAY "You much changed . Marriage change you ...."  '[voice 4, skip 1]
[436B]       CLEAR concept_alt
    END
[436C]     IF-BLOCK (exit -> @43A6)
[436F]       GUARD concept == "jimmy"
[4372]       ENDIF
[4373]       SAY "Me greet you , friend Jimi"  '[voice 2]
[4387]       SAY "You want eat guitar ? Me have delicious electric guitars ..."  '[voice 3, skip 1]
[43A5]       CLEAR concept_alt
    END
[43A6]     IF-BLOCK (exit -> @43E0)
[43A9]       GUARD concept == "billy"
[43AC]       ENDIF
[43AD]       SAY "Me greet you , friend Billy"  '[voice 2]
[43C1]       SAY "You not have twisted mouth today ? You have accident ????"  '[voice 2, skip 1]
[43DF]       CLEAR concept_alt
    END
[43E0]     SAY "Me MIGRATOR, great musician ... Important artist ..."  '[voice 2]
[43F8]     SAY "Me look for singer for group . Make concert . You can help ?"  '[voice 3]
[441C]     IF-BLOCK (exit -> @4467)
[441F]       GUARD (rec_0C9A & 0x2) != 0
[4424]       ENDIF
[4425]       SAY "Me know you know Tina Burner . She sing at Purple Haze , on planet Eden ..."  '[voice 2]
[444F]       SAY "Me like Tina voice . Very sexy ...."  '[voice 1]
    END
[4467]     SAY "Me must rehearse . Bye bye , friend . You come back when want ..."  '[voice 4]
[448D]     SAY "Bye bye friend..."  '[skip 4]
[449B]     K1 = 1
[44A2]     trak10 = 1
[44A9]     POKE [0x423D] = 0
[44AD]     END PRESENTATION Migrator.talk
  END
[44B0]   BLOCK (exit -> @454F)
[44B4]     AWAIT gameflag_252A
[44B5]     GUARD rec_088A == 3278
[44BA]     GUARD K1 == 1
[44C1]     GUARD rec_1088 == 4100
[44C6]     GUARD active_actor == Migrator.talk (related 40)
[44CB]     ENDIF
[44CC]     SAY "Migrator happy to see you again , friend ..."  '[voice 2]
[44E6]     SAY "You did find singer ?"  '[voice 3]
[44F8]     SAY "Me want be star , friend ... Me look for singer ."  '[voice 4]
[4518]     SAY "See you soon , friend . Migrator must rehearse music for concert ..."  '[voice 1]
[453A]     SAY "Bye bye , friend ..."  '[voice 3, skip 1]
[454C]     END PRESENTATION Migrator.talk
  END
[454F]   BLOCK (exit -> @4632)
[4553]     AWAIT gameflag_252A
[4554]     GUARD K1 == 1
[455B]     GUARD rec_1088 == 4100
[4560]     GUARD active_actor == Migrator.talk (related 40)
[4565]     GUARD rec_088A == 65535
[456A]     ENDIF
[456B]     SAY "Friend ... You be here ... Me happy see you ..."
[4589]     SAY "Me know you know Tina Burner..."
[459D]     SAY "You teleport Tina Burner. Me know her ... She sing at PURPLE HAZE on planet EDEN"  '[voice 2]
[45C5]     SAY "Let's get rid of her , Commander ..."
[45DD]     SAY "TELEPORT TINA BURNER TO AIRPORT word_65535 teleport"
[45F5]     IF-BLOCK (exit -> @460F)
[45F8]       GUARD concept == "teleport"
[45FB]       ENDIF
[45FC]       SAY "TELEPORTING TINA BURNER TO MOSKITO"  '[skip 1]
[460E]       CLEAR concept_alt
    END
[460F]     SAY "Thank you , friend ... Me be happy ...."  '[skip 2]
[4629]     rec_088A = 4070
[462E]     POKE [0x4550] = 0
  END
[4632]   BLOCK (exit -> @4704)
[4636]     AWAIT gameflag_252A
[4637]     GUARD rec_088A == 4070
[463C]     GUARD rec_1088 == 4100
[4641]     GUARD active_actor == Migrator.talk (related 40)
[4646]     ENDIF
[4647]     SAY "Ohhh thank you , Commander . You're a sweetypoo !!!"
[4663]     SAY "Such a cutie ... It's so nice here ..."
[467D]     SAY "He's called AMIGO ..."
[468D]     SAY "Hey , sex fiend ! Tone it down , okay ?"
[46AB]     SAY "Where shall I put myself ?"
[46BF]     SAY "Thanks , Commander . Something tells me we're going to get along just fine ..."  '[voice 2]
[46E5]     SAY "See you ..."  '[voice 5]
[46F3]     SAY "..."  '[skip 2]
[46FD]     POKE [0x4633] = 0
[4701]     END PRESENTATION Migrator.talk
  END
[4704]   BLOCK (exit -> @47A1)
[4708]     AWAIT gameflag_252A
[4709]     GUARD rec_088A == 4070
[470E]     GUARD active_actor == Migrator.talk (related 40)
[4713]     GUARD rec_1088 == 4100
[4718]     ENDIF
[4719]     SAY "Greetings ... Me happy see you back ..."  '[voice 3]
[4731]     SAY "We rehearse music , friend . We soon do concert ..."  '[voice 5]
[474F]     SAY "Tina be great singer ..."  '[voice 2]
[4761]     SAY "Me go see Tina in recording studio ..."  '[voice 5]
[4779]     SAY "See you soon ... Happy landings ..."  '[voice 3]
[478F]     SAY "..."  '[skip 2]
[4799]     rec_052A = 4070
[479E]     END PRESENTATION Migrator.talk
  END
[47A1]   GOTO @4939
[47A5]   AWAIT gameflag_252A
[47A6]   GUARD rec_088A == 4070
[47AB]   GUARD rec_1088 == 4100
[47B0]   GUARD active_actor == Migrator.talk (related 40)
[47B5]   ENDIF
[47B6]   SAY "Me greet you , friend ..."  '[voice 5]
[47CA]   SAY "You have ring ??? ..."  '[voice 2]
[47DC]   SAY "Me soon marry TINA... YOU COME WEDDING , ME INVITE YOU , WE DO BIG CONCERT ..."  '[voice 3]
[4806]   IF-BLOCK (exit -> @48E2)
[4809]     GUARD rec_137A == 40
[480E]     ENDIF
[480F]     SAY "Commander, teleport him the ring . Make him happy ...."
[482B]     SAY "TELEPORT RING TO MIGRATOR word_65535 TELEPORT"
[4841]     IF-BLOCK (exit -> @4866)
[4844]       GUARD concept == "TELEPORT"
[4847]       ENDIF
[4848]       SAY "RING TELEPORTED TO MOSKITO AIRPORT ZONE ."  '[skip 2]
[485E]       OP_CD CD 30 00 66 13 12 05
[4865]       CLEAR concept_alt
    END
[4866]     SAY "WOW ! Nice nice ring ... Me like ... Me thank you , friend ...."  '[voice 2]
[488C]     SAY "Thank you ... thank you ... FRIEND YOU COME WEDDING ... WE DO BIG CONCERT ..."  '[voice 3]
[48B4]     SAY "BYE BYE FRIEND .... TINA SENDS YOU KISS ..."  '[voice 2]
[48CE]     SAY "..."  '[skip 2]
[48D8]     mariage = 1
[48DF]     END PRESENTATION Migrator.talk
  END
[48E2]   SAY "You not have ring ? Me worried now , friend ..."  '[voice 2]
[4900]   SAY "You go quick get ring ..."  '[voice 3]
[4914]   SAY "Me wait . Bye bye , friend ..."  '[voice 2]
[492C]   SAY "..."  '[skip 1]
[4936]   END PRESENTATION Migrator.talk
[4939]   BLOCK (exit -> @4953)
[493D]     AWAIT presentation
[493E]     GUARD B1 == 2
[4945]     ENDIF
[4946]     OP_C3 C3 D4 07 28 00
[494B]     POKE [0x4954] = 1
[494F]     POKE [0x493A] = 0
  END
[4953]   GOTO @4AAD
[4957]   AWAIT presentation
[4958]   ENDIF
[4959]   SAY "Hello hello ... HANNA SCRUTA here . Husband HEKTOR be fighter pilot ..."
[497B]   SAY "You blow up Hektor fighter in combat ... ME WIDOW NOW ... CRY ... CRY ..."
[49A3]   SAY "ME NOT CAN PAY RENT ... YOU FAULT ... INSURANCE NOT PAY ..."
[49C5]   SAY "ME WIDOW ... CRY ... CRY ... DESPAIRING HOWLS .... YOU KILL HEKTOR ..."
[49E9]   SAY "Commander, they're giving out your phone number to widows now ..."
[4A07]   SAY "It's not in the rules , Commander . They're trying to psych us out . Don't let 'em get to you ..."
[4A3B]   SAY "YOU MUST PAY ... CRY ... CRY ... YOU KILL MY HEKTOR ... CRY ... CRY ...."
[4A65]   SAY "KRUIKKK..."
[4A6F]   SAY "Phew ! She just hung up ... This feels bad to me , Commander ..."
[4A95]   SAY "stop"  '[skip 3]
[4A9F]   trak18 = 1
[4AA6]   POKE [0x4954] = 0
[4AAA]   END PRESENTATION Scruter_K.talk
[4AAD]   BLOCK (exit -> @4C26)
[4AB1]     AWAIT gameflag_252A
[4AB2]     GUARD C1 == 0
[4AB9]     GUARD active_actor == Scruter_Mac.talk (related 40)
[4ABE]     ENDIF
[4ABF]     SAY "YOU IN FORBIDDEN ZONE ... ME ISSUE WARNING"  '[voice 18]
[4AD7]     SAY "Me not know you ."  '[voice 17]
[4AE9]     SAY "Commander , it's Scruter Mac . I can tell by the smell ..."
[4B0B]     SAY "He doesn't recognize us , Commander . Very weird ..."
[4B27]     SAY "You give code ..."  '[voice 16]
[4B37]     SAY "Quick , stranger ! word_65535 1 2 3 4 code 6 7 8 9 0"  '[voice 1]
[4B5F]     IF-BLOCK (exit -> @4BA1)
[4B62]       GUARD concept == "code"
[4B65]       ENDIF
[4B66]       SAY "Code have changed . Code not code now . Code too easy ..."  '[voice 12]
[4B88]       SAY "Me suspect you ... Me not like ..."  '[voice 14, skip 1]
[4BA0]       CLEAR concept_alt
    END
[4BA1]     IF-BLOCK (exit -> @4BBC)
[4BA4]       GUARD NOT concept == "code"
[4BA8]       ENDIF
[4BA9]       SAY "You did forget code ..."  '[voice 2, skip 1]
[4BBB]       CLEAR concept_alt
    END
[4BBC]     SAY "You make funny face , friend ..."  '[voice 6]
[4BD2]     SAY "Ha ! Ha ! Me kid you .... Ha ! Ha ! ...."  '[voice 2]
[4BF4]     SAY "Me let you pass . You can see prisoner ..."  '[voice 3]
[4C10]     SAY "..."  '[skip 2]
[4C1A]     C1 = 1
[4C21]     OP_C1 C1 D0 13 1E 0B
  END
[4C26]   BLOCK (exit -> @4CB5)
[4C2A]     AWAIT gameflag_252A
[4C2B]     GUARD C1 == 1
[4C32]     GUARD rec_02A2 == 2846
[4C37]     GUARD rec_1088 == 2792
[4C3C]     GUARD active_actor == Scruter_Mac.talk (related 40)
[4C41]     ENDIF
[4C42]     SAY "Hello , friend ..."  '[voice 5]
[4C52]     SAY "You take care , friend . Croolis Eviscerator be very dangerous ..."  '[voice 6]
[4C72]     SAY "Me open door ..."  '[voice 7]
[4C82]     SAY "Maybe we should stand clear , Commander . Call it a survival technique ..."
[4CA6]     SAY "..."  '[skip 1]
[4CB0]     OP_C1 C1 D0 13 1E 0B
  END
[4CB5]   BLOCK (exit -> @4D52)
[4CB9]     AWAIT gameflag_252A
[4CBA]     GUARD NOT rec_02A2 == 2846
[4CC0]     GUARD rec_1088 == 2792
[4CC5]     GUARD active_actor == Scruter_Mac.talk (related 40)
[4CCA]     ENDIF
[4CCB]     SAY "You take care . Croolis Eviscerator did escape . Did blow up cell ..."  '[voice 6]
[4CEF]     SAY "Commander , he blew his cell away ... Think about it ... Your splatch ..."
[4D15]     SAY "You leave . SCRUT be nervous now ... Bye bye , friend ... QUICK ..."  '[voice 5]
[4D3B]     SAY "stop"  '[skip 3]
[4D45]     rec_076A = 4070
[4D4A]     rec_07B2 = 3332
[4D4F]     END PRESENTATION Scruter_Mac.talk
  END
[4D52]   BLOCK (exit -> @4F5E)
[4D56]     AWAIT gameflag_252A
[4D57]     GUARD D1 == 0
[4D5E]     GUARD active_actor == Eviscerator.talk (related 40)
[4D63]     GUARD rec_1088 == 2792
[4D68]     ENDIF
[4D69]     SAY "Who you be , stranger . You want see me ?"  '[voice 5]
[4D87]     IF-BLOCK (exit -> @4DDC)
[4D8A]       GUARD rec_02C0 == 1
[4D91]       ENDIF
[4D92]       SAY "Me big EVISCERATOR CROOLIS ... Ha ! Ha ! Ha !"  '[voice 6]
[4DB0]       SAY "Yikes , Commander , you know what I'm saying ? ... This guy gives me undesirable feelings ..."
    END
[4DDC]     SAY "WHAT YOU WANT , STRANGER ?"  '[voice 7, skip 1]
[4DF0]     rec_02D0 = 1
[4DF5]     IF-BLOCK (exit -> @4E24)
[4DF8]       GUARD secret == 1
[4DFF]       ENDIF
[4E00]       SAY "You like secrets ? ... Me tell secrets"  '[voice 6, skip 2]
[4E18]       rec_02D0 = 9683
[4E1D]       secret = 0
    END
[4E24]     IF-BLOCK (exit -> @4F37)
[4E27]       GUARD secret1 == 1
[4E2E]       ENDIF
[4E2F]       SAY "SPLATCH be very concentrated explosive . SPLATCH obliterate jail .. LAUGH . DISGUSTING SWEAR ..."  '[voice 10]
[4E55]       SAY "You get me SPLATCH , friend ... And me tell you hiding place treasure ..."  '[voice 8]
[4E7B]       SAY "Commander , did he just growl what I distinctly heard growling ? ... Ol' Turkeyface Bob's gonna love this !..."
[4EAB]       SAY "You go see friends at "Purple Haze" on planet Eden ... Them help you ..."  '[voice 5]
[4ED1]       SAY "Planet Eden in Edenus galaxy , position x4532 y6754 ..."  '[voice 6, skip 1]
[4EED]       rec_0C9A |= 0x2
[4EF2]       SAY "Notch up another planet , Commander ..."
[4F08]       SAY "You understand ? Bye bye . Me wait you impatiently ..."  '[voice 9, skip 3]
[4F26]       secret1 = 0
[4F2D]       D1 = 1
[4F34]       END PRESENTATION Eviscerator.talk
    END
[4F37]     SAY "Bye bye . You come back soon . me like visits ... word_65535 bye_bye"  '[voice 6, skip 1]
[4F5B]     END PRESENTATION Eviscerator.talk
  END
[4F5E]   BLOCK (exit -> @50BB)
[4F62]     AWAIT gameflag_252A
[4F63]     GUARD rec_1088 == 3332
[4F68]     GUARD rec_07B2 == 3332
[4F6D]     GUARD active_actor == Scruter_K.talk (related 40)
[4F72]     ENDIF
[4F73]     SAY "RLA... RLA... LDIR AX,AY... PUSH A... POP MUSIC... GHA... GHA...."  '[voice 0, skip 1]
[4F8F]     LOADSTR "mag_scr.hnm"
[4F9D]     SAY "Holy hamstrings ! Commander , he's lost it ... It's our pal Scruter Mac . I recognize the perfume ..."
[4FCD]     SAY "They switched him off , Commander . It has to be because he let EVISCERATOR get away from Mastachok..."
[4FFB]     SAY "Cd blood... blood.exe... Eviscerator equals Fatal error ... error ... error ..."  '[voice 1]
[501B]     SAY "Teleport him into the Ark , Commander ? word_65535 teleport refuse"
[503B]     IF-BLOCK (exit -> @5066)
[503E]       GUARD concept == "teleport"
[5041]       ENDIF
[5042]       SAY "TELEPORT SCRUTER MAC'S BODY TO ARK"  '[skip 4]
[5056]       OP_CD CD D4 07 3E 11 28 00
[505D]       rec_07B2 = 4070
[5062]       CLEAR concept_alt
[5063]       END PRESENTATION Scruter_K.talk
    END
[5066]     IF-BLOCK (exit -> @50BB)
[5069]       GUARD concept == "refuse"
[506C]       ENDIF
[506D]       SAY "Poor Mister Scruter Mac ... What a way to go ... A minute's silence ..."
[5093]       SAY "PEACE BE UPON HIS LUBRICANT NOZZLES ... CRY ..."
[50AD]       SAY "..."  '[skip 2]
[50B7]       CLEAR concept_alt
[50B8]       END PRESENTATION Scruter_K.talk
    END
  END
[50BB]   BLOCK (exit -> @526C)
[50BF]     AWAIT gameflag_252A
[50C0]     GUARD D1 == 1
[50C7]     GUARD rec_13CA == 2846
[50CC]     GUARD active_actor == Eviscerator.talk (related 40)
[50D1]     ENDIF
[50D2]     SAY "You back ... You bring splatch ???"  '[voice 8]
[50E8]     SAY "Me big EVISCERATOR CROOLIS . Me want escape ... CRY ... CRY ... GNASH ..."  '[voice 1]
[510E]     SAY "CRY ... GNASH ..."  '[voice 1]
[511E]     IF-BLOCK (exit -> @517A)
[5121]       GUARD rec_131A == 40
[5126]       ENDIF
[5127]       SAY "Give him the splatch , Commander... We need that treasure ..."
[5145]       SAY "TELEPORT SPLATCH TO EVISCERATOR word_65535 teleport"
[515B]       IF-BLOCK (exit -> @517A)
[515E]         GUARD concept == "teleport"
[5161]         ENDIF
[5162]         SAY "TELEPORTING SPLATCH TO EVISCERATOR"  '[skip 2]
[5172]         OP_CD CD 30 00 06 13 8A 02
[5179]         CLEAR concept_alt
      END
    END
[517A]     IF-BLOCK (exit -> @5205)
[517D]       GUARD rec_131A == 650
[5182]       ENDIF
[5183]       SAY "You want know where is treasure ? ..."  '[voice 6]
[519B]       SAY "Treasure be on planet TUMUL..."  '[voice 8]
[51AD]       SAY "Coordinates AX329 Tumulus constellation ..."  '[voice 5]
[51BF]       SAY "You do realize he just told us where to find the treasure , Commander ... I love you , Commander ."  '[skip 4]
[51F1]       rec_0D7E |= 0x2
[51F6]       D1 = 2
[51FD]       rec_02A2 = 4070
[5202]       END PRESENTATION Eviscerator.talk
    END
[5205]     IF-BLOCK (exit -> @526C)
[5208]       GUARD NOT rec_131A == 40
[520E]       GUARD NOT rec_131A == 650
[5214]       ENDIF
[5215]       SAY "INSULT ... MIGHTY SWEAR ... CRY ... GNASH ... SNARL ..."  '[voice 10]
[5233]       SAY "ME HATE YOUR GUT ... INSULT ...."  '[voice 12]
[5249]       SAY "ME NOT SAY BYE BYE ..."  '[voice 11]
[525D]       SAY "Bye bye"  '[skip 1]
[5269]       END PRESENTATION Eviscerator.talk
    END
  END
[526C]   BLOCK (exit -> @52AE)
[5270]     AWAIT gameflag_252A
[5271]     GUARD D1 == 2
[5278]     GUARD active_actor == Eviscerator.talk (related 40)
[527D]     GUARD rec_1088 == 2792
[5282]     ENDIF
[5283]     SAY "What you want ?"  '[voice 5]
[5293]     SAY "ME NOT SAY BYE BYE ... word_65535 bye_bye"  '[voice 6, skip 1]
[52AB]     END PRESENTATION Eviscerator.talk
  END
[52AE]   BLOCK (exit -> @52D4)
[52B2]     GUARD D1 == 2
[52B9]     GUARD NOT rec_088A == 3278
[52BF]     GUARD NOT rec_06DA == 3278
[52C5]     GUARD rec_131A == 650
[52CA]     ENDIF
[52CB]     rec_02A2 = 3278
[52D0]     POKE [0x52AF] = 0
  END
[52D4]   BLOCK (exit -> @5352)
[52D8]     AWAIT gameflag_252A
[52D9]     GUARD active_actor == Eviscerator.talk (related 40)
[52DE]     GUARD rec_1088 == 3224
[52E3]     ENDIF
[52E4]     SAY "You here ... SWEAR ... INSULT ..."  '[voice 1]
[52FA]     SAY "YOU FOLLOW ME ... SWEAR ... INSULT ..."  '[voice 1]
[5312]     SAY "You meet your maker .... Bye bye ..."  '[voice 2]
[532A]     SAY "BOOOOOMMM !!!"  '[skip 4]
[5336]     LOADSTR "explo3.hnm"
[5343]     rec_02A2 = 4070
[5348]     D1 = 4
[534F]     END PRESENTATION Eviscerator.talk
  END
[5352]   BLOCK (exit -> @54DE)
[5356]     AWAIT gameflag_252A
[5357]     GUARD active_actor == t10 (related 40)
[535C]     GUARD beau == 0
[5363]     ENDIF
[5364]     IF-BLOCK (exit -> @53F8)
[5367]       GUARD NOT rec_1152 == 40
[536D]       GUARD NOT rec_1152 == 1442
[5373]       ENDIF
[5374]       SAY "Ahh ! A stranger ... Help ... Help ..."  '[voice 10]
[538E]       SAY "Commander , I think he wants help ... Uh oh ... He's badly burned ..."  '[skip 1]
[53B4]       rec_05E8 = 10242
[53B9]       SAY "Aaaahhh ... Bye ... bye ... word_65535 bye_bye"  '[voice 11]
[53D1]       SAY "He's in bad shape , Commander ... We better help him .... word_65535 bye_bye"  '[skip 1]
[53F5]       END PRESENTATION t10
    END
[53F8]     IF-BLOCK (exit -> @54DE)
[53FB]       GUARD rec_1152 == 40
[5400]       ENDIF
[5401]       SAY "Ahh ! A stranger ... Help ... Help ..."  '[voice 10]
[541B]       SAY "Commander , I think he wants help ... Uh oh ... He's badly burned ..."
[5441]       SAY "He's in bad shape , Commander ... We better help him ...."
[5461]       SAY "Give him SCRUTER MAC's body , Commander ..."
[5479]       SAY "TELEPORT SCRUTER MAC'S BODY TO PLANET TUMUL word_65535 teleport refuse"
[5497]       IF-BLOCK (exit -> @54BF)
[549A]         GUARD concept == "teleport"
[549D]         ENDIF
[549E]         SAY "TELEPORTING SCRUTER BODY TO TUMUL"  '[skip 3]
[54B0]         OP_CD CD 30 00 3E 11 A2 05
[54B7]         beau = 1
[54BE]         CLEAR concept_alt
      END
[54BF]       IF-BLOCK (exit -> @54DE)
[54C2]         GUARD concept == "refuse"
[54C5]         ENDIF
[54C6]         SAY "Whatever you say , Commander ..."  '[skip 2]
[54DA]         CLEAR concept_alt
[54DB]         END PRESENTATION t10
      END
    END
  END
[54DE]   BLOCK (exit -> @5646)
[54E2]     AWAIT gameflag_252A
[54E3]     GUARD active_actor == t10 (related 40)
[54E8]     GUARD beau == 1
[54EF]     ENDIF
[54F0]     SAY "Thank you , thank you ... Ahh , I feel better , Commander . I guess I owe you my life ..."  '[voice 0]
[5524]     SAY "Thank you thank you ..."  '[voice 1]
[5536]     SAY "What do you want to know , Commander ?"  '[voice 2, skip 1]
[5550]     rec_05E8 = 10281
[5555]     IF-BLOCK (exit -> @55A6)
[5558]       GUARD poem == 1
[555F]       ENDIF
[5560]       SAY "Ata ata Hoglo hulu"  '[voice 3]
[5570]       SAY "Herr tot Zaglo holo hulu"  '[voice 4]
[5582]       SAY "Ata ata haglo holo hulu"  '[voice 5]
[5594]       SAY "Hamm tot zurglo holo hulu"  '[voice 3]
    END
[55A6]     IF-BLOCK (exit -> @55F5)
[55A9]       GUARD delir == 1
[55B0]       ENDIF
[55B1]       SAY "Commander , he's inherited Scruter Mac's fantasies ... Ha! ha! Ha!"
[55CF]       SAY "It must be some of the old Scruter's memory that didn't want to die ..."
    END
[55F5]     IF-BLOCK (exit -> @5646)
[55F8]       GUARD regard == 1
[55FF]       ENDIF
[5600]       SAY "I'm going to reveal the secrets of planet TUMUL to you , Commander..."  '[voice 1]
[5622]       SAY "Follow me ..."  '[voice 8, skip 4]
[5630]       rec_05BA = 3506
[5635]       rec_0842 = 3452
[563A]       beau = 2
[5641]       OP_C1 C1 D0 13 B2 0D
    END
  END
[5646]   BLOCK (exit -> @56B4)
[564A]     AWAIT gameflag_252A
[564B]     GUARD active_actor == Fifi.talk (related 40)
[5650]     ENDIF
[5651]     SAY "Ca.."  '[voice 1]
[565B]     SAY "Cake..."  '[voice 2]
[5665]     SAY "... Hee Hee Hee ..."  '[voice 0]
[5677]     SAY "Heh heh... That guy's funny, Commander ..."
[568D]     SAY "Me like gift... Eat ... Eat..."  '[voice 1]
[56A1]     SAY "Bye bye... Me go..."  '[voice 5, skip 1]
[56B1]     END PRESENTATION Fifi.talk
  END
[56B4]   BLOCK (exit -> @5760)
[56B8]     AWAIT gameflag_252A
[56B9]     GUARD rec_13CA == 3506
[56BE]     GUARD active_actor == t10 (related 40)
[56C3]     GUARD beau == 2
[56CA]     ENDIF
[56CB]     SAY "Check out these archeological wonders !"  '[voice 1]
[56DF]     SAY "A people called the PATAGOS lived here long ago ..."  '[voice 2]
[56FB]     SAY "Until their sun , GLADIS , exploded ..."  '[voice 3]
[5713]     SAY "I'm looking for the tomb of BETAKAM IV , king of the PATAGOS..."  '[voice 4]
[5735]     SAY "Follow me , Commander..."  '[voice 5]
[5745]     SAY "..."  '[skip 3]
[574F]     rec_05BA = 3530
[5754]     OP_C1 C1 D0 13 CA 0D
[5759]     beau = 3
  END
[5760]   BLOCK (exit -> @580F)
[5764]     AWAIT gameflag_252A
[5765]     GUARD rec_13CA == 3530
[576A]     GUARD active_actor == t10 (related 40)
[576F]     GUARD beau == 3
[5776]     GUARD NOT rec_131A == 40
[577C]     ENDIF
[577D]     SAY "The tomb of BETAKAM IV lies beneath this rubble ..."  '[voice 1]
[5799]     SAY "I need dynamite , or any explosive that can blow away the rubble ..."  '[voice 2]
[57BD]     SAY "Find me some explosive , Commander ..."  '[voice 3]
[57D3]     SAY "I shall continue working ..."  '[voice 4]
[57E5]     SAY "What he needs is SPLATCH , Commander ..."
[57FD]     SAY "..."  '[skip 2]
[5807]     rec_0DCC |= 0x2
[580C]     END PRESENTATION t10
  END
[580F]   BLOCK (exit -> @5937)
[5813]     AWAIT gameflag_252A
[5814]     GUARD active_actor == t10 (related 40)
[5819]     GUARD rec_13CA == 3530
[581E]     GUARD beau == 3
[5825]     ENDIF
[5826]     SAY "The tomb of BETAKAM IV lies beneath this rubble ..."  '[voice 1]
[5842]     SAY "I need dynamite , or any explosive that can blow away the rubble ..."  '[voice 2]
[5866]     SAY "You have the explosive , Commander ?"  '[voice 1]
[587C]     IF-BLOCK (exit -> @5913)
[587F]       GUARD rec_131A == 40
[5884]       ENDIF
[5885]       SAY "Let's teleport him the SPLATCH , Commander ..."
[589D]       SAY "TELEPORT SPLATCH TO TUMUL word_65535 teleport refuse"
[58B5]       IF-BLOCK (exit -> @58DB)
[58B8]         GUARD concept == "teleport"
[58BB]         ENDIF
[58BC]         SAY "TELEPORTING SPLATCH TO TUMUL"  '[skip 3]
[58CC]         OP_CD CD 30 00 06 13 A2 05
[58D3]         beau = 4
[58DA]         CLEAR concept_alt
      END
[58DB]       IF-BLOCK (exit -> @5913)
[58DE]         GUARD concept == "refuse"
[58E1]         ENDIF
[58E2]         SAY "Whatever you say , Commander . But allow me to say you're a big dummy ..."  '[skip 3]
[590A]         rec_0DCC |= 0x2
[590F]         CLEAR concept_alt
[5910]         END PRESENTATION t10
      END
    END
[5913]     IF-BLOCK (exit -> @5937)
[5916]       GUARD NOT rec_131A == 40
[591C]       ENDIF
[591D]       SAY "I need this SPLATCH, Commander..."  '[voice 1, skip 2]
[592F]       rec_0DCC |= 0x2
[5934]       END PRESENTATION t10
    END
  END
[5937]   BLOCK (exit -> @59C2)
[593B]     AWAIT gameflag_252A
[593C]     GUARD rec_13CA == 3530
[5941]     GUARD active_actor == t10 (related 40)
[5946]     GUARD beau == 4
[594D]     ENDIF
[594E]     SAY "Thanks , Commander . You're a standup guy ..."  '[voice 4]
[5968]     SAY "I'm going to make this rubble sorry it was ever born ..."  '[voice 8]
[5988]     SAY "Stand back , Commander ..."  '[voice 7, skip 1]
[599A]     LOADSTR "explo3.hnm"
[59A7]     SAY "..."  '[skip 3]
[59B1]     rec_05BA = 3554
[59B6]     beau = 5
[59BD]     OP_C1 C1 D0 13 E2 0D
  END
[59C2]   BLOCK (exit -> @5AEA)
[59C6]     AWAIT gameflag_252A
[59C7]     GUARD rec_13CA == 3554
[59CC]     GUARD active_actor == t10 (related 40)
[59D1]     GUARD beau == 5
[59D8]     ENDIF
[59D9]     SAY "Holy Handstands ... Get a load of that , Commander ..."  '[voice 4]
[59F7]     SAY "I love those mummies ... Such beauty ... Such splendor ..."  '[voice 5]
[5A15]     SAY "Commander, to reward you , I offer you the mummy of BETAKAM IV ..."  '[voice 6]
[5A39]     SAY "You earned it ... As for me , I shall study this astonishing site ..."  '[voice 7]
[5A5F]     SAY "TELEPORT MUMMY TO ARK word_65535 teleport"
[5A75]     IF-BLOCK (exit -> @5AEA)
[5A78]       GUARD concept == "teleport"
[5A7B]       ENDIF
[5A7C]       SETCHAR slot 1 = "maledict"
[5A88]       SETCHAR slot 2 = "maledict"
[5A94]       SETCHAR slot 3 = "maledict"
[5AA0]       SETCHAR slot 4 = "maledict"
[5AAC]       SETCHAR slot 5 = "maledict"
[5AB8]       SETCHAR slot 6 = "maledict"
[5AC4]       SAY "TELEPORTING MUMMY TO ARK"  '[skip 4]
[5AD4]       OP_CD CD DC 05 6E 11 28 00
[5ADB]       maledict = 1
[5AE2]       CLEAR concept_alt
[5AE3]       beau = 6
    END
  END
[5AEA]   BLOCK (exit -> @5BEB)
[5AEE]     AWAIT gameflag_252A
[5AEF]     GUARD active_actor == t10 (related 40)
[5AF4]     GUARD beau == 6
[5AFB]     ENDIF
[5AFC]     SAY "AAAAAHHH, Commander ... I am cursed ...."  '[voice 23]
[5B12]     SAY "AAAAAAAAAAAAAAAAAAAAAAA !!!"  '[voice 24]
[5B1E]     SAY "OOOOOOOOOOOOOOOOOOOOOOO !!!"  '[voice 25]
[5B2A]     SAY "THE .... THE CURSE !!!"  '[voice 30]
[5B3C]     SAY "hhhhhhhhhhh"  '[voice 29]
[5B46]     SAY "Holy handmaidens , Commander ! He caught the curse of the mummy ... Couldn't happen to a nicer guy ... ..."
[5B78]     SAY "Better teleport him aboard for treatment , Commander . We can't leave him like that ..."
[5BA0]     SAY "TELEPORT BEAUREGARD TO ARK word_65535 teleport"
[5BB6]     IF-BLOCK (exit -> @5BEB)
[5BB9]       GUARD concept == "teleport"
[5BBC]       ENDIF
[5BBD]       SAY "aaaaaaaaa"  '[voice 28]
[5BC7]       SAY "TELEPORTING BEAUREGARD TO ARK"  '[skip 5]
[5BD7]       rec_05BA = 65535
[5BDC]       rec_0DCC &= !0x2
[5BE2]       rec_0DE4 |= 0x2
[5BE7]       CLEAR concept_alt
[5BE8]       END PRESENTATION t10
    END
  END
[5BEB]   BLOCK (exit -> @5C81)
[5BEF]     AWAIT gameflag_274F
[5BF0]     GUARD active_actor == t10 (related 40)
[5BF5]     GUARD rec_05BA == 65535
[5BFA]     ENDIF
[5BFB]     SAY "Aaaaaa, Commander I'm blasted..."  '[voice 23]
[5C0B]     SAY "AAAAAAAAAA!!!"  '[voice 24]
[5C15]     SAY "OOOOOOOOhh!!!"  '[voice 25]
[5C1F]     SAY "B b bLasteD..."  '[voice 30, skip 1]
[5C2D]     LOADSTR "maledict.hnm"
[5C3C]     SAY "hhhhhhhh"  '[voice 29, skip 1]
[5C46]     LOADSTR "maledict.hnm"
[5C55]     SAY "Holy handguns , Commander ... He caught the curse of the mummy !..."  '[skip 2]
[5C77]     fion = 1
[5C7E]     END PRESENTATION t10
  END
[5C81]   BLOCK (exit -> @5C9B)
[5C85]     GUARD NOT rec_088A == 3278
[5C8B]     GUARD NOT rec_02A2 == 3278
[5C91]     ENDIF
[5C92]     rec_06DA = 3278
[5C97]     POKE [0x5C82] = 0
  END
[5C9B]   BLOCK (exit -> @5DFC)
[5C9F]     AWAIT gameflag_252A
[5CA0]     GUARD rec_06DA == 3278
[5CA5]     GUARD active_actor == Amigo.talk (related 40)
[5CAA]     GUARD rec_1088 == 3224
[5CAF]     GUARD M1 == 0
[5CB6]     ENDIF
[5CB7]     SAY "HALT STRANGER !!!"  '[voice 3]
[5CC5]     SAY "Do you know any of our customers ??? The PURPLE HAZE doesn't accept just anybody ... word_65535 Morning_Oil Eviscerator Izwalito Bronko Tina_Burner"  '[voice 2]
[5CFB]     IF-BLOCK (exit -> @5D32)
[5CFE]       GUARD concept == "Morning_Oil"
[5D01]       ENDIF
[5D02]       SAY "I don't know any Morning Oil , stranger . You'd better leave if you don't want trouble ...."  '[voice 4, skip 2]
[5D2E]       CLEAR concept_alt
[5D2F]       END PRESENTATION Amigo.talk
    END
[5D32]     IF-BLOCK (exit -> @5D6F)
[5D35]       GUARD concept == "Tina_Burner"
[5D38]       ENDIF
[5D39]       SAY "That old bag Tina Burner ! She's got a nerve sending people here ... Out of my sight , stranger ..."  '[voice 3, skip 2]
[5D6B]       CLEAR concept_alt
[5D6C]       END PRESENTATION Amigo.talk
    END
[5D6F]     IF-BLOCK (exit -> @5DA4)
[5D72]       GUARD concept == "Izwalito"
[5D75]       ENDIF
[5D76]       SAY "I don't know any Izwalito , stranger . You'd better leave if you don't want trouble ........"  '[voice 4, skip 2]
[5DA0]       CLEAR concept_alt
[5DA1]       END PRESENTATION Amigo.talk
    END
[5DA4]     IF-BLOCK (exit -> @5DD9)
[5DA7]       GUARD concept == "Bronko"
[5DAA]       ENDIF
[5DAB]       SAY "I don't know any Bronko , stranger . You'd better leave if you don't want trouble ...."  '[voice 4, skip 2]
[5DD5]       CLEAR concept_alt
[5DD6]       END PRESENTATION Amigo.talk
    END
[5DD9]     IF-BLOCK (exit -> @5DFC)
[5DDC]       GUARD concept == "Eviscerator"
[5DDF]       ENDIF
[5DE0]       SAY "EVISCERATOR !!! Eviscerator sent you ?"  '[voice 4, skip 2]
[5DF4]       evi = 1
[5DFB]       CLEAR concept_alt
    END
  END
[5DFC]   BLOCK (exit -> @5F9B)
[5E00]     AWAIT gameflag_252A
[5E01]     GUARD active_actor == Amigo.talk (related 40)
[5E06]     GUARD rec_1088 == 3224
[5E0B]     GUARD evi == 1
[5E12]     ENDIF
[5E13]     SAY "The password ,"  '[voice 3]
[5E21]     SAY "You know the PASSWORD , stranger ? word_65535 croolas meteorut galabar Mastachok"  '[voice 2]
[5E43]     IF-BLOCK (exit -> @5E5D)
[5E46]       GUARD concept == "croolas"
[5E49]       ENDIF
[5E4A]       SAY "CROOLAS GETS YOU EVERYTIME ...."  '[voice 2, skip 1]
[5E5C]       CLEAR concept_alt
    END
[5E5D]     IF-BLOCK (exit -> @5E7B)
[5E60]       GUARD concept == "meteorut"
[5E63]       ENDIF
[5E64]       SAY "THE SCARLET METEORITE RUPTURES YOUR SPLEEN ...."  '[voice 2, skip 1]
[5E7A]       CLEAR concept_alt
    END
[5E7B]     IF-BLOCK (exit -> @5E97)
[5E7E]       GUARD concept == "galabar"
[5E81]       ENDIF
[5E82]       SAY "GALABAR BARMEN MAKE KILLER COCKTAILS ...."  '[voice 2, skip 1]
[5E96]       CLEAR concept_alt
    END
[5E97]     IF-BLOCK (exit -> @5EB7)
[5E9A]       GUARD concept == "Mastachok"
[5E9D]       ENDIF
[5E9E]       SAY "MASTAR OF MASTACHOK MUNCHES MOUNTAINS OF MEAT ....."  '[voice 2, skip 1]
[5EB6]       CLEAR concept_alt
    END
[5EB7]     SAY "What do you want , stranger ?"  '[voice 3, skip 2]
[5ECD]     secret = 0
[5ED4]     rec_0708 = 1
[5ED9]     SAY "Bye bye , stranger ... word_65535 bye_bye"  '[voice 3, skip 1]
[5EEF]     END PRESENTATION Amigo.talk
[5EF2]     IF-BLOCK (exit -> @5F9B)
[5EF5]       GUARD secret == 1
[5EFC]       ENDIF
[5EFD]       SAY "Be careful , stranger . That stuff is sensitive ..."  '[voice 5]
[5F19]       SAY "TELEPORT SPLATCH TO ARK . word_65535 teleport refuse"
[5F33]       IF-BLOCK (exit -> @5F59)
[5F36]         GUARD concept == "teleport"
[5F39]         ENDIF
[5F3A]         SAY "SPLATCH TELEPORTED TO ARK ..."  '[skip 3]
[5F4C]         OP_CD CD FC 06 06 13 28 00
[5F53]         rec_06DA = 4070
[5F58]         CLEAR concept_alt
      END
[5F59]       IF-BLOCK (exit -> @5F9B)
[5F5C]         GUARD concept == "refuse"
[5F5F]         ENDIF
[5F60]         SAY "You can refuse if you want ..."  '[voice 2]
[5F76]         SAY "Don't hang around here ... It's dangerous ..."  '[voice 5]
[5F8E]         SAY "..."  '[skip 1]
[5F98]         END PRESENTATION Amigo.talk
      END
    END
  END
[5F9B]   BLOCK (exit -> @5FB3)
[5F9F]     GUARD NOT rec_02A2 == 2846
[5FA5]     ENDIF
[5FA6]     rec_0332 = 3116
[5FAB]     POKE [0x5FB4] = 1
[5FAF]     POKE [0x5F9C] = 0
  END
[5FB3]   GOTO @61E5
[5FB7]   AWAIT gameflag_252A
[5FB8]   GUARD rec_1088 == 3116
[5FBD]   GUARD active_actor == Yoko.talk (related 40)
[5FC2]   GUARD H1 == 0
[5FC9]   ENDIF
[5FCA]   rec_0360 = 12
[5FCF]   SAY "Ah! You there , Commander . Me scared ... CRY CRY"  '[voice 1]
[5FED]   SAY "Father Maxxon be kidnapped . Them want big ransom ... CRY ... CRY ..."  '[voice 2]
[6011]   SAY "Me not can pay ... CRY ..."  '[voice 3]
[6027]   SAY "Holy headaches ! Commander, you gotta do something ..."
[6041]   IF-BLOCK (exit -> @6082)
[6044]     GUARD rec_0350 == 1
[604B]     ENDIF
[604C]     SAY "Me think kidnapper be Croolis ..."  '[voice 3]
[6060]     SAY "Croolis EVISCERATOR did escape jail on planet Mastachok ... Him be mean ..."  '[voice 4]
  END
[6082]   SAY "Me scared , Commander ..."  '[voice 5]
[6094]   IF-BLOCK (exit -> @60CF)
[6097]     GUARD NOT rec_03C2 == 65535
[609D]     GUARD NOT rec_025A == 65535
[60A3]     ENDIF
[60A4]     SAY "Me afraid Commander..."  '[voice 2]
[60B2]     SAY "You go fast..."  '[voice 5]
[60C0]     SAY "Bye bye.."  '[voice 0, skip 1]
[60CC]     END PRESENTATION Yoko.talk
  END
[60CF]   IF-BLOCK (exit -> @61A5)
[60D2]     GUARD rec_03C2 == 65535
[60D7]     GUARD rec_025A == 65535
[60DC]     ENDIF
[60DD]     SAY "We can't leave him alone , Commander ... Morning_Oil could protect him ..."
[60FF]     SAY "How about teleporting Morning to the planet Rondo ?"
[6119]     SAY "TELEPORT MORNING OIL TO PLANET RONDO word_65535 teleport refuse"
[6135]     IF-BLOCK (exit -> @6156)
[6138]       GUARD concept == "teleport"
[613B]       ENDIF
[613C]       SAY "TELEPORTING MORNING OIL TO PLANET RONDO"  '[skip 2]
[6150]       rec_03C2 = 3170
[6155]       CLEAR concept_alt
    END
[6156]     IF-BLOCK (exit -> @61A5)
[6159]       GUARD concept == "refuse"
[615C]       ENDIF
[615D]       SAY "You're running the show , Commander ..."
[6173]       SAY "You not leave me alone , friend Commander... CRY CRY ... Me scared ..."  '[voice 5]
[6197]       SAY "..."  '[skip 2]
[61A1]       CLEAR concept_alt
[61A2]       END PRESENTATION Yoko.talk
    END
  END
[61A5]   IF-BLOCK (exit -> @61E5)
[61A8]     GUARD rec_03C2 == 3170
[61AD]     ENDIF
[61AE]     SAY "Thank you Commander ... You nice ..."  '[voice 10]
[61C4]     SAY "Commander, let's go check out the observatory ..."  '[skip 2]
[61DC]     POKE [0x5FB4] = 0
[61E0]     OP_C1 C1 D0 13 62 0C
  END
[61E5]   BLOCK (exit -> @6293)
[61E9]     AWAIT gameflag_252A
[61EA]     GUARD rec_1088 == 3116
[61EF]     GUARD rec_03C2 == 3170
[61F4]     GUARD active_actor == Morning_Oil.talk (related 40)
[61F9]     ENDIF
[61FA]     SAY "I understand my mission , Commander . I must protect the young Izwal ..."  '[voice 1]
[621E]     SAY "Have no fear ... Have confidence in me ..."  '[voice 2]
[6238]     SAY "I completely reprogrammed him , Commander . You can trust him ..."
[6258]     SAY "See you soon ... I'll call you on the radio if there's a problem ..."  '[voice 3]
[627E]     SAY "..."  '[skip 3]
[6288]     POKE [0x61E6] = 0
[628C]     POKE [0x5FB4] = 0
[6290]     END PRESENTATION Morning_Oil.talk
  END
[6293]   BLOCK (exit -> @6320)
[6297]     AWAIT gameflag_252A
[6298]     GUARD rec_1088 == 3116
[629D]     GUARD active_actor == Yoko.talk (related 40)
[62A2]     GUARD rec_0332 == 3116
[62A7]     GUARD rec_03C2 == 3170
[62AC]     ENDIF
[62AD]     SAY "Everything's fine , Commander . Nothing to report ... Morning_Oil is very nice ..."  '[voice 7]
[62D1]     SAY "He's in the observatory , playing with my father's telescope ..."  '[voice 6]
[62EF]     SAY "Go and see him . He'd like that ..."  '[voice 7]
[6309]     SAY "..."  '[skip 3]
[6313]     POKE [0x6294] = 0
[6317]     POKE [0x6321] = 1
[631B]     OP_C1 C1 D0 13 62 0C
  END
[6320]   GOTO @636D
[6324]   AWAIT gameflag_252A
[6325]   GUARD active_actor == Yoko.talk (related 40)
[632A]   GUARD rec_03C2 == 3170
[632F]   ENDIF
[6330]   SAY "Ah! You here . Commander , me afraid ... CRY CRY"  '[voice 1]
[634E]   SAY "Me very worried ..."  '[voice 4]
[635E]   SAY "Bye bye..."  '[voice 2, skip 1]
[636A]   END PRESENTATION Yoko.talk
[636D]   BLOCK (exit -> @645F)
[6371]     AWAIT gameflag_252A
[6372]     GUARD rec_1088 == 3116
[6377]     GUARD active_actor == Morning_Oil.talk (related 40)
[637C]     GUARD rec_0332 == 3116
[6381]     GUARD rec_03C2 == 3170
[6386]     ENDIF
[6387]     SAY "Ah! Hello Commander... Are you okay ?"  '[voice 4]
[639D]     SAY "I've made some interesting discoveries ..."  '[voice 5]
[63B1]     SAY "I've been observing the sky with Mister maxxon's telescope ..."  '[voice 6]
[63CD]     SAY "And I've found a new planet ..."  '[voice 7]
[63E3]     SAY "Planet VULCAN , coordinates x342 y543..."  '[voice 3, skip 1]
[63F7]     rec_0F52 |= 0x2
[63FC]     SAY "Good work Mister Morning ... See , Commander ! I programmed him good ..."
[6420]     SAY "See you soon ... I'll call you on the radio if there's a problem ..."  '[voice 3]
[6446]     SAY "..."  '[skip 4]
[6450]     state[9] = 70
[6454]     POKE [0x6460] = 1
[6458]     POKE [0x636E] = 0
[645C]     END PRESENTATION Morning_Oil.talk
  END
[645F]   GOTO @6473
[6463]   GUARD state[9] == 0
[6465]   ENDIF
[6466]   OP_C3 C3 E4 03 28 00
[646B]   POKE [0x6474] = 1
[646F]   POKE [0x6460] = 0
[6473]   GOTO @650D
[6477]   AWAIT presentation
[6478]   GUARD active_actor == Morning_Oil.talk (related 40)
[647D]   ENDIF
[647E]   SAY "Commander ? ... Come in Commander ... This is MORNING OIL ..."
[649E]   SAY "I have news ... An investigator just arrived on Rondo ..."
[64BC]   SAY "You better come , Commander . The situation requires it ..."
[64DA]   SAY "I'll be expecting you ..."
[64EC]   SAY "Cruikk ..."
[64F8]   SAY "..."  '[skip 3]
[6502]   POKE [0x650E] = 1
[6506]   POKE [0x6474] = 0
[650A]   END PRESENTATION Morning_Oil.talk
[650D]   GOTO @652A
[6511]   GUARD NOT rec_1088 == 3116
[6517]   ENDIF
[6518]   rec_03C2 = 4070
[651D]   rec_040A = 3170
[6522]   POKE [0x652B] = 1
[6526]   POKE [0x650E] = 0
[652A]   GOTO @65AE
[652E]   AWAIT gameflag_252A
[652F]   GUARD rec_1088 == 3116
[6534]   GUARD active_actor == Yoko.talk (related 40)
[6539]   GUARD rec_040A == 3170
[653E]   ENDIF
[653F]   SAY "Ah! You here . Commander , me afraid ... CRY CRY"  '[voice 1]
[655D]   SAY "Me very worried ... You know INSPECTOR JERRY KHAN ?"  '[voice 4]
[6579]   SAY "Inspector Jerry Khan investigate Eviscerator ..."  '[voice 5]
[658D]   SAY "You go Observatory see Inspector Jerry Khan ..."  '[voice 3, skip 2]
[65A5]   POKE [0x65AF] = 1
[65A9]   OP_C1 C1 D0 13 62 0C
[65AE]   GOTO @6710
[65B2]   AWAIT gameflag_252A
[65B3]   GUARD rec_1088 == 3116
[65B8]   GUARD active_actor == Jerry_Khan.talk (related 40)
[65BD]   ENDIF
[65BE]   SAY "Hello , Commander . Your robot Morning Oil has told me a lot about you ..."  '[voice 1]
[65E6]   SAY "Mister Maxxon has been kidnapped , probably by the unspeakable EVISCERATOR ..."  '[voice 2]
[6606]   SAY "Some GLUXX children have also disappeared ..."  '[voice 3]
[661C]   SAY "I don't like mysteries ..."  '[voice 4]
[662E]   SAY "I inspected Mister Maxxon's telescope ... The lens is most unusual ..."  '[voice 5]
[664E]   SAY "It reminds me of a Slimer lens ..."  '[voice 6]
[6666]   SAY "Did you know , Commander , that doctor Otto von Smile has also vanished ?"  '[voice 7]
[668C]   SAY "Too many mysteries for my taste ..."  '[voice 5]
[66A2]   SAY "I'll take young Yoko with me in my ship , the SHARK. He'll be safe ..."  '[voice 1]
[66CA]   SAY "I'll be in contact with you ..."  '[voice 2]
[66E0]   SAY "See you soon , Commander..."  '[voice 3]
[66F2]   SAY "..."  '[skip 4]
[66FC]   rec_040A = 4270
[6701]   rec_0332 = 4070
[6706]   jerry = 1
[670D]   END PRESENTATION Jerry_Khan.talk
[6710]   BLOCK (exit -> @671D)
[6714]     ENDIF
[6715]     state[16] = 50
[6719]     POKE [0x6711] = 0
  END
[671D]   BLOCK (exit -> @6734)
[6721]     GUARD state[16] == 0
[6723]     GUARD rec_0590 == 0
[672A]     ENDIF
[672B]     OP_C3 C3 94 05 28 00
[6730]     POKE [0x671E] = 0
  END
[6734]   BLOCK (exit -> @6790)
[6738]     AWAIT presentation
[6739]     GUARD active_actor == Daddy_Gluxx.talk (related 40)
[673E]     ENDIF
[673F]     SAY "Come in Commander ... Help !!!"
[6753]     SAY "Come quick to our planet Ekatomb ... Something's happened ..."
[676F]     SAY "I'll be waiting for you ..."
[6783]     SAY "..."  '[skip 1]
[678D]     END PRESENTATION Daddy_Gluxx.talk
  END
[6790]   BLOCK (exit -> @6922)
[6794]     AWAIT gameflag_252A
[6795]     GUARD L1 == 0
[679C]     GUARD rec_1088 == 4040
[67A1]     GUARD active_actor == Daddy_Gluxx.talk (related 40)
[67A6]     ENDIF
[67A7]     SAY "Ho! The Visitor from distant space-time ..."  '[voice 9]
[67BD]     IF-BLOCK (exit -> @689A)
[67C0]       GUARD rec_0590 < 3
[67C7]       ENDIF
[67C8]       SAY "You have come back , stranger ... Something unfortunate has occurred ..."  '[voice 9]
[67E8]       SAY "My children , Gelatine , Rubber , Gooseberry , Latex ... They have been kidnapped ..."  '[voice 9]
[6810]       SAY "They've disappeared ... DISAPPEARED ... CRY ... CRY ..."  '[voice 9]
[682A]       SAY "They're no longer on planet Ekatomb ... Someone took them ..."  '[voice 9]
[6848]       SAY "Commander , why does the hateful doctor Otto Von smile spring to mind ?"
[686C]       SAY "Oh no !!! It's all our fault , Commander ... We told the doctor about the planet EKATOMB !"
    END
[689A]     IF-BLOCK (exit -> @6905)
[689D]       GUARD rec_0590 > 2
[68A4]       ENDIF
[68A5]       SAY "Do you have news , stranger ... I am sick with worry ..."  '[voice 9]
[68C7]       SAY "CRY ... CRY ..."  '[voice 9]
[68D7]       SAY "Find them , Commander . I beg you ..."  '[voice 9]
[68F1]       SAY "Commander ... This is awful ..."
    END
[6905]     SAY "Bye bye ... help us ..."  '[skip 2]
[6919]     rec_055C &= !0x2
[691F]     END PRESENTATION Daddy_Gluxx.talk
  END
[6922]   GOTO @6943
[6926]   AWAIT gameflag_252A
[6927]   GUARD rec_1088 == 2684
[692C]   GUARD active_actor == Otto_Von_Smile.talk (related 40)
[6931]   GUARD P1 == 0
[6938]   ENDIF
[6939]   SAY "hello"  '[voice 2]
[6943]   BLOCK (exit -> @695B)
[6947]     GUARD B11 == 1
[694E]     ENDIF
[694F]     state[7] = 20
[6953]     OP_B7 B7 D0 01 02
[6957]     POKE [0x6944] = 0
  END
[695B]   BLOCK (exit -> @6A03)
[695F]     AWAIT presentation
[6960]     GUARD active_actor == Bug_Deluxe.talk (related 40)
[6965]     ENDIF
[6966]     SAY "COMMERCIAL ... COMMERCIAL ... COMMERCIAL ..."
[697A]     SAY "BRAND NEW STORE EVEN BIGGER TOTALLY BETTER ..."
[6992]     SAY "SATISFY YOUR SHOPPING CRAVINGS . COME TO VENUSIA"
[69AA]     SAY "MEGABARGAINS GALORE . BRING CREDS ..."
[69BE]     SAY "YOU'LL LOVE SPENDING CREDS AT VENUSIAAAAA ..."
[69D4]     SAY "VENUSIA 325467 DEGREES , 23 GALAXY B BABY1..."
[69EC]     SAY "CRUIK..........."
[69F6]     SAY "stop"  '[skip 1]
[6A00]     END PRESENTATION Bug_Deluxe.talk
  END
[6A03]   BLOCK (exit -> @6AA5)
[6A07]     GUARD rec_1088 == 2594
[6A0C]     GUARD active_actor == Bug_Deluxe.talk (related 40)
[6A11]     ENDIF
[6A12]     SAY "Hi there , Consumer Comrade . Welcome to VENUSIA SUPRAMARKET !"  '[voice 1]
[6A30]     SAY "wanna buy a SUPRA product from VENUSIA ? word_65535 yes no"  '[voice 4]
[6A50]     IF-BLOCK (exit -> @6A5C)
[6A53]       GUARD concept == "yes"
[6A56]       ENDIF
[6A57]       POKE [0x6AA6] = 1
[6A5B]       CLEAR concept_alt
    END
[6A5C]     IF-BLOCK (exit -> @6AA5)
[6A5F]       GUARD concept == "no"
[6A62]       ENDIF
[6A63]       SAY "NO ! HA HA !! left your creds at home or what ?"  '[voice 6]
[6A85]       SAY "Bye bye , comrade ..."  '[voice 2]
[6A97]       SAY "stop"  '[skip 2]
[6AA1]       CLEAR concept_alt
[6AA2]       END PRESENTATION Bug_Deluxe.talk
    END
  END
[6AA5]   GOTO @6C4F
[6AA9]   GUARD rec_1088 == 2594
[6AAE]   AWAIT gameflag_252A
[6AAF]   GUARD active_actor == Bug_Deluxe.talk (related 40)
[6AB4]   ENDIF
[6AB5]   SAY "You got the creds ? Everything costs creds at VENUSIA , Consumer Comrade ..."  '[voice 5]
[6AD9]   IF-BLOCK (exit -> @6B22)
[6ADC]     GUARD PP1 == -1.value
[6AE3]     GUARD ach == -1.value
[6AEA]     GUARD NOT rec_119A == 40
[6AF0]     ENDIF
[6AF1]     SAY "You don't have creds ? We don't care for poor clients ..."
[6B11]     SAY "stop"  '[skip 2]
[6B1B]     POKE [0x6AA6] = 0
[6B1F]     END PRESENTATION Bug_Deluxe.talk
  END
[6B22]   IF-BLOCK (exit -> @6BCC)
[6B25]     GUARD PP1 == 1
[6B2C]     ENDIF
[6B2D]     SAY "It's your lucky day , consumer comrade . We're happy to offer you a FREE CRED !!"  '[voice 5]
[6B57]     SAY "TELEPORT CRED TO ARK word_65535 teleport"
[6B6D]     IF-BLOCK (exit -> @6BCC)
[6B70]       GUARD concept == "teleport"
[6B73]       ENDIF
[6B74]       SAY "TELEPORTING CRED TO ARK"  '[skip 1]
[6B84]       LOADSTR "parf1_2.hnm"
[6B92]       SAY "Come back soon to VENUSIA , SPACES'S LEADING SUPRA MARKET ..."  '[voice 5]
[6BB0]       SAY "stop"  '[skip 4]
[6BBA]       PP1 = -1.value
[6BC1]       OP_CD CD EC 01 86 11 28 00
[6BC8]       CLEAR concept_alt
[6BC9]       END PRESENTATION Bug_Deluxe.talk
    END
  END
[6BCC]   IF-BLOCK (exit -> @6BFA)
[6BCF]     GUARD rec_119A == 40
[6BD4]     ENDIF
[6BD5]     SAY "What would you care to buy , supra comrade ?"  '[voice 2, skip 2]
[6BF1]     state[7] = 100
[6BF5]     rec_01F8 = 12402
  END
[6BFA]   IF-BLOCK (exit -> @6C08)
[6BFD]     GUARD state[7] == 0
[6BFF]     ENDIF
[6C00]     state[7] = 65535
[6C04]     POKE [0x6C50] = 1
  END
[6C08]   IF-BLOCK (exit -> @6C4F)
[6C0B]     GUARD ach == 1
[6C12]     ENDIF
[6C13]     SAY "thank you for visiting VENUSIA the SUPRAMARKET . Come again soon to VENUSIA ..."
[6C37]     SAY "stop"  '[skip 3]
[6C41]     ach = -1.value
[6C48]     POKE [0x6AA6] = 0
[6C4C]     END PRESENTATION Bug_Deluxe.talk
  END
[6C4F]   GOTO @6C90
[6C53]   AWAIT gameflag_252A
[6C54]   ENDIF
[6C55]   SAY "We're closing right now ..."
[6C67]   SAY "Please move to the nearest cashpoint..."
[6C7B]   SAY "stop"  '[skip 3]
[6C85]   POKE [0x6AA6] = 0
[6C89]   POKE [0x6C50] = 0
[6C8D]   END PRESENTATION Bug_Deluxe.talk
[6C90]   BLOCK (exit -> @6CC6)
[6C94]     GUARD fish == 1
[6C9B]     GUARD fion == 1
[6CA2]     GUARD rec_13C2 == 40
[6CA7]     GUARD rec_088A == 4070
[6CAC]     GUARD rec_06DA == 4070
[6CB1]     GUARD jerry == 1
[6CB8]     ENDIF
[6CB9]     OP_C3 C3 2C 04 28 00
[6CBE]     POKE [0x6CC7] = 1
[6CC2]     POKE [0x6C91] = 0
  END
[6CC6]   GOTO @6E07
[6CCA]   AWAIT presentation
[6CCB]   GUARD active_actor == Jerry_Khan.talk (related 40)
[6CD0]   ENDIF
[6CD1]   SAY "Come in , Commander . This is Inspector JERRY KHAN onboard the SHARK..."
[6CF3]   SAY "I'm currently pursuing EVISCERATOR and Doctor Otto von Smile..."
[6D0D]   SAY "They've just entered a black hole called ODDLAND ..."
[6D27]   SAY "I need your help . Your ship is tougher than mine ..."
[6D47]   SAY "Oddland is at coordinates x465 Y342 ..."  '[skip 1]
[6D5D]   rec_1054 |= 0x2
[6D62]   SAY "Howzabout that for a twist , Commander !"
[6D7A]   SAY "See you soon, Commander . we'll meet up on the other side of the BLACK HOLE ..."
[6DA4]   SAY "This is Yoko : save my father !!!"
[6DBC]   SAY "If you should forfeit your life , you'll be promoted to KNIGHT OF THE GUILD , posthumously ..."
[6DE8]   SAY "GOODBYE COMMANDER ..."
[6DF6]   SAY "..."  '[skip 2]
[6E00]   POKE [0x6E08] = 1
[6E04]   END PRESENTATION Jerry_Khan.talk
[6E07]   GOTO @6E13
[6E0B]   OP_C6 C6 8E 10 52 10
[6E10]   ENDIF
[6E11]   RUN PROFILE 3
[6E13]   BLOCK (exit -> @6EC1)
[6E17]     AWAIT presentation
[6E18]     GUARD active_actor == menu.talk (related 40)
[6E1D]     ENDIF
[6E1E]     SAY ""MENU""
[6E28]     SAY "Today's mouthwatering meal , courtesy of your CHEF :"
[6E42]     SAY "HONK-style PLASMA soup ."
[6E52]     SAY "WRIGGLER guts with DROOLER sauce ."
[6E66]     SAY "MURFFALO marrow in jellied URTIKAN ."
[6E7A]     SAY "GLOK-eye pie ."
[6E88]     SAY "Recycled water"
[6E94]     SAY "Your chef says : eat it while it's hot !"
[6EB0]     SAY "Stop"  '[skip 2]
[6EBA]     POKE [0x6E14] = 0
[6EBE]     END PRESENTATION menu.talk
  END
[6EC1]   BLOCK (exit -> @6F71)
[6EC5]     AWAIT presentation
[6EC6]     GUARD active_actor == menu.talk (related 40)
[6ECB]     ENDIF
[6ECC]     SAY ""MENU""
[6ED6]     SAY "Today's yummy menu , brought to you by your CHEF :"
[6EF4]     SAY "Honk-style PLASMA drip ."
[6F04]     SAY "WRIGGLER snout in body fluid ."
[6F18]     SAY "URTIKAN nuts with poached MURFFALO lung ."
[6F2E]     SAY "GLOK-milk yoghurt ."
[6F3C]     SAY "Recycled water"
[6F48]     SAY "Your CHEF says : You lucky eaters !"
[6F60]     SAY "stop"  '[skip 2]
[6F6A]     POKE [0x6EC2] = 0
[6F6E]     END PRESENTATION menu.talk
  END
[6F71]   BLOCK (exit -> @7025)
[6F75]     AWAIT presentation
[6F76]     GUARD active_actor == menu.talk (related 40)
[6F7B]     ENDIF
[6F7C]     SAY ""MENU""
[6F86]     SAY "And here's what your CHEF has imagined for today's meal :"
[6FA4]     SAY "HONK-style curdled PLASMA ."
[6FB4]     SAY "WRIGGLER feet in emulsive gravy ."
[6FC8]     SAY "URTIKAN leaves in MURFFALO juice ."
[6FDC]     SAY "GLOK-petal ice-cream ."
[6FEA]     SAY "Recycled water"
[6FF6]     SAY "your CHEF says : No eating with your mouth full !"
[7014]     SAY "stop"  '[skip 2]
[701E]     POKE [0x6F72] = 0
[7022]     END PRESENTATION menu.talk
  END
[7025]   BLOCK (exit -> @70E7)
[7029]     AWAIT presentation
[702A]     GUARD active_actor == menu.talk (related 40)
[702F]     ENDIF
[7030]     SAY ""MENU""
[703A]     SAY "Take a look at what your CHEF has prepared for your delight :"
[705C]     SAY "HONK-style PLASMA thick soup ."
[706E]     SAY "WRIGGLER brain simmered in its own fluid ."
[7086]     SAY "URTIKAN trunk braised in reconstituted MURFFALO spleen ."
[709E]     SAY "GLOK dee-lite ."
[70AC]     SAY "Recycled water"
[70B8]     SAY "Your CHEF says : No talking with your mouth open !"
[70D6]     SAY "stop"  '[skip 2]
[70E0]     POKE [0x7026] = 0
[70E4]     END PRESENTATION menu.talk
  END
[70E7]   BLOCK (exit -> @71A1)
[70EB]     AWAIT presentation
[70EC]     GUARD active_actor == menu.talk (related 40)
[70F1]     ENDIF
[70F2]     SAY ""NEW IMPROVED MENU""
[7100]     SAY "Today's menu is brought to you by your CHEF :"
[711C]     SAY "HONK-style PLASMA soup ."
[712C]     SAY "WRIGGLER alphabet kidneys deep-fried in natural perspiration ."
[7144]     SAY "URTIKAN root barbecued in recycled oil ."
[715A]     SAY "GLOK surprise ."
[7168]     SAY "Recycled water"
[7174]     SAY "Your CHEF says : Ventriloquists never burp ..."
[718C]     SAY "stop"  '[skip 3]
[7196]     POKE [0x70E8] = 0
[719A]     POKE [0x71A2] = 1
[719E]     END PRESENTATION menu.talk
  END
[71A1]   GOTO @71BE
[71A5]   ENDIF
[71A6]   POKE [0x6E14] = 1
[71AA]   POKE [0x6EC2] = 1
[71AE]   POKE [0x6F72] = 1
[71B2]   POKE [0x7026] = 1
[71B6]   POKE [0x70E8] = 1
[71BA]   POKE [0x71A2] = 0
[71BE]   BLOCK (exit -> @73E0)
[71C2]     AWAIT presentation
[71C3]     GUARD active_actor == Honk.talk (related 40)
[71C8]     ENDIF
[71C9]     SAY "I exist only to obey , Commander"
[71DF]     IF-BLOCK (exit -> @7252)
[71E2]       GUARD vbio == 0
[71E9]       ENDIF
[71EA]       SAY "Commander , we don't have any BIONIUM ... COMMANDER , please ..."
[720A]       SAY "I need that energy ..."
[721C]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
[7232]       SAY "Wake up Scruter_Jo , Commander . He's sleeping in the Cryobox ..."
    END
[7252]     IF-BLOCK (exit -> @72B3)
[7255]       GUARD vbio == 1
[725C]       ENDIF
[725D]       SAY "We've got one dose of BIONIUM left , Commander"
[7277]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
[728D]       SAY "I don't feel too sure of myself , Commander... I really need that energy ..."
    END
[72B3]     IF-BLOCK (exit -> @72EE)
[72B6]       GUARD vbio == 2
[72BD]       ENDIF
[72BE]       SAY "We've got two doses of BIONIUM left , Commander"
[72D8]       SAY "You must enter Scruter Jo's CYBERSPACE ..."
    END
[72EE]     IF-BLOCK (exit -> @7313)
[72F1]       GUARD vbio == 3
[72F8]       ENDIF
[72F9]       SAY "We've got three doses of BIONIUM left , Commander"
    END
[7313]     IF-BLOCK (exit -> @7338)
[7316]       GUARD vbio == 4
[731D]       ENDIF
[731E]       SAY "We've got four doses of BIONIUM left , Commander"
    END
[7338]     IF-BLOCK (exit -> @735D)
[733B]       GUARD vbio == 5
[7342]       ENDIF
[7343]       SAY "We've got five doses of BIONIUM left , Commander"
    END
[735D]     IF-BLOCK (exit -> @7382)
[7360]       GUARD vbio == 6
[7367]       ENDIF
[7368]       SAY "We've got six doses of BIONIUM left , Commander"
    END
[7382]     IF-BLOCK (exit -> @73A7)
[7385]       GUARD vbio == 7
[738C]       ENDIF
[738D]       SAY "We've got seven doses of BIONIUM left , Commander"
    END
[73A7]     IF-BLOCK (exit -> @73E0)
[73AA]       GUARD vbio == 8
[73B1]       ENDIF
[73B2]       SAY "We've got eight doses of BIONIUM left , Commander"
[73CC]       SAY "You're the best , Commander ..."
    END
  END
[73E0]   BLOCK (exit -> @7471)
[73E4]     AWAIT presentation
[73E5]     GUARD vbio > 2
[73EC]     GUARD (rec_0C9A & 0x2) == 0
[73F2]     GUARD rec_02A2 == 2846
[73F7]     GUARD active_actor == Honk.talk (related 40)
[73FC]     ENDIF
[73FD]     SAY "Commander, go see Eviscerator in his jail and talk to him about WAR and treasure ..."
[7425]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[743F]     vbio -= 3
[7446]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[746A]     POKE [0x73E1] = 0
[746E]     END PRESENTATION Honk.talk
  END
[7471]   BLOCK (exit -> @74F2)
[7475]     AWAIT presentation
[7476]     GUARD vbio > 2
[747D]     GUARD rec_0590 == 0
[7484]     GUARD active_actor == Honk.talk (related 40)
[7489]     ENDIF
[748A]     SAY "Commander, you forgot to visit Daddy_Gluxx on the planet Ekatomb..."
[74A6]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[74C0]     vbio -= 3
[74C7]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[74EB]     POKE [0x7472] = 0
[74EF]     END PRESENTATION Honk.talk
  END
[74F2]   BLOCK (exit -> @7578)
[74F6]     AWAIT presentation
[74F7]     GUARD vbio > 2
[74FE]     GUARD rec_0470 == 0
[7505]     GUARD rec_0452 == 65535
[750A]     GUARD active_actor == Honk.talk (related 40)
[750F]     ENDIF
[7510]     SAY "Commander, you ought to talk to Bronko in the cryobox..."
[752C]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7546]     vbio -= 3
[754D]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[7571]     POKE [0x74F3] = 0
[7575]     END PRESENTATION Honk.talk
  END
[7578]   BLOCK (exit -> @7624)
[757C]     AWAIT presentation
[757D]     GUARD vbio > 2
[7584]     GUARD rec_0452 == 65535
[7589]     GUARD NOT rec_131A == 40
[758F]     GUARD brk == 0
[7596]     GUARD active_actor == Honk.talk (related 40)
[759B]     ENDIF
[759C]     SAY "Commander , you teleported Bronko to spy on Erazor ..."
[75B8]     SAY "To teleport him, approach Erazor and talk to Bronko in the cryobox"
[75D8]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[75F2]     vbio -= 3
[75F9]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[761D]     POKE [0x7579] = 0
[7621]     END PRESENTATION Honk.talk
  END
[7624]   BLOCK (exit -> @76A9)
[7628]     AWAIT presentation
[7629]     GUARD vbio > 2
[7630]     GUARD rec_03C2 == 4246
[7635]     GUARD active_actor == Honk.talk (related 40)
[763A]     ENDIF
[763B]     SAY "Commander, Morning Oil is still in the Ark... he's a great mechanic, remember..."
[765D]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7677]     vbio -= 3
[767E]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[76A2]     POKE [0x7625] = 0
[76A6]     END PRESENTATION Honk.talk
  END
[76A9]   BLOCK (exit -> @772C)
[76AD]     AWAIT presentation
[76AE]     GUARD vbio > 2
[76B5]     GUARD rec_1392 == 65535
[76BA]     GUARD active_actor == Honk.talk (related 40)
[76BF]     ENDIF
[76C0]     SAY "Commander, I'm pretty sure Tina Burner would love to have that guitar..."
[76E0]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[76FA]     vbio -= 3
[7701]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[7725]     POKE [0x76AA] = 0
[7729]     END PRESENTATION Honk.talk
  END
[772C]   BLOCK (exit -> @77B1)
[7730]     AWAIT presentation
[7731]     GUARD vbio > 2
[7738]     GUARD rec_088A == 65535
[773D]     GUARD active_actor == Honk.talk (related 40)
[7742]     ENDIF
[7743]     SAY "Commander,Tina and Migrator would make a great couple on the planet Moskito ..."
[7765]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[777F]     vbio -= 3
[7786]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[77AA]     POKE [0x772D] = 0
[77AE]     END PRESENTATION Honk.talk
  END
[77B1]   BLOCK (exit -> @7842)
[77B5]     AWAIT presentation
[77B6]     GUARD vbio > 2
[77BD]     GUARD NOT rec_088A == 3278
[77C3]     GUARD NOT rec_131A == 40
[77C9]     GUARD rec_02A2 == 2846
[77CE]     GUARD active_actor == Honk.talk (related 40)
[77D3]     ENDIF
[77D4]     SAY "Commander, go see Amigo on the planet Eden and talk about his itches..."
[77F6]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7810]     vbio -= 3
[7817]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[783B]     POKE [0x77B2] = 0
[783F]     END PRESENTATION Honk.talk
  END
[7842]   BLOCK (exit -> @78C9)
[7846]     AWAIT presentation
[7847]     GUARD vbio > 2
[784E]     GUARD rec_04E2 == 3332
[7853]     GUARD active_actor == Honk.talk (related 40)
[7858]     ENDIF
[7859]     SAY "Commander, take a trip to the planet Magnus... There may be other broken robots..."
[787D]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7897]     vbio -= 3
[789E]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[78C2]     POKE [0x7843] = 0
[78C6]     END PRESENTATION Honk.talk
  END
[78C9]   BLOCK (exit -> @794A)
[78CD]     AWAIT presentation
[78CE]     GUARD vbio > 2
[78D5]     GUARD rec_0308 == 0
[78DC]     GUARD active_actor == Honk.talk (related 40)
[78E1]     ENDIF
[78E2]     SAY "Commander, Mister Hom may be back on the planet Kortex..."
[78FE]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7918]     vbio -= 3
[791F]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[7943]     POKE [0x78CA] = 0
[7947]     END PRESENTATION Honk.talk
  END
[794A]   BLOCK (exit -> @79DB)
[794E]     AWAIT presentation
[794F]     GUARD vbio > 2
[7956]     GUARD rec_05D8 > 0
[795D]     GUARD NOT rec_1152 == 1442
[7963]     GUARD active_actor == Honk.talk (related 40)
[7968]     ENDIF
[7969]     SAY "Commander, let's go visit Magnus... Maybe we'll find a new body for Mister Beauregard ..."
[798F]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[79A9]     vbio -= 3
[79B0]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[79D4]     POKE [0x794B] = 0
[79D8]     END PRESENTATION Honk.talk
  END
[79DB]   BLOCK (exit -> @7A5D)
[79DF]     AWAIT presentation
[79E0]     GUARD vbio > 2
[79E7]     GUARD rec_0350 == 0
[79EE]     GUARD rec_0332 == 3116
[79F3]     GUARD active_actor == Honk.talk (related 40)
[79F8]     ENDIF
[79F9]     SAY "Commander, Yoko may be on the planet Rondo..."
[7A11]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7A2B]     vbio -= 3
[7A32]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[7A56]     POKE [0x79DC] = 0
[7A5A]     END PRESENTATION Honk.talk
  END
[7A5D]   BLOCK (exit -> @7AE7)
[7A61]     AWAIT presentation
[7A62]     GUARD vbio > 2
[7A69]     GUARD bronk4 == 1082
[7A6E]     GUARD NOT rec_0452 == 2684
[7A74]     GUARD fish == 0
[7A7B]     GUARD active_actor == Honk.talk (related 40)
[7A80]     ENDIF
[7A81]     SAY "Commander, listen to the receiver in the cryobox ..."
[7A9B]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7AB5]     vbio -= 3
[7ABC]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[7AE0]     POKE [0x7A5E] = 0
[7AE4]     END PRESENTATION Honk.talk
  END
[7AE7]   BLOCK (exit -> @7B64)
[7AEB]     AWAIT presentation
[7AEC]     GUARD vbio > 2
[7AF3]     GUARD rec_05BA == 65535
[7AF8]     GUARD active_actor == Honk.talk (related 40)
[7AFD]     ENDIF
[7AFE]     SAY "Commander, talk to Mister Beauregard in the cryobox ..."
[7B18]     SAY "That help sure eats up our BIONIUM reserves, Commander..."  '[skip 1]
[7B32]     vbio -= 3
[7B39]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[7B5D]     POKE [0x7AE8] = 0
[7B61]     END PRESENTATION Honk.talk
  END
[7B64]   BLOCK (exit -> @7B80)
[7B68]     AWAIT presentation
[7B69]     GUARD active_actor == Honk.talk (related 40)
[7B6E]     ENDIF
[7B6F]     SAY "See you, Commander..."  '[skip 1]
[7B7D]     END PRESENTATION Honk.talk
  END
[7B80]   GOTO @7BF3
[7B84]   AWAIT presentation
[7B85]   GUARD active_actor == Scruter_K.talk (related 40)
[7B8A]   ENDIF
[7B8B]   SAY ""  '[skip 1]
[7B93]   SS = 1
[7B9A]   SAY ""  '[skip 1]
[7BA2]   SS = 2
[7BA9]   SAY ""  '[skip 1]
[7BB1]   SS = 3
[7BB8]   SAY ""  '[skip 1]
[7BC0]   SS = 4
[7BC7]   SAY ""  '[skip 1]
[7BCF]   SS = 5
[7BD6]   IF-BLOCK (exit -> @7BE4)
[7BD9]     GUARD SS == 0
[7BE0]     ENDIF
[7BE1]     GOTO @7B8B
  END
[7BE4]   IF-BLOCK (exit -> @7BF3)
[7BE7]     GUARD SS > 0
[7BEE]     ENDIF
[7BEF]     POKE [0x7B81] = 0
  END
[7BF3]   BLOCK (exit -> @7C00)
[7BF7]     ENDIF
[7BF8]     state[12] = 1500
[7BFC]     POKE [0x7BF4] = 0
  END
[7C00]   BLOCK (exit -> @7C16)
[7C04]     ENDIF
[7C05]     IF-BLOCK (exit -> @7C16)
[7C08]       GUARD state[12] == 0
[7C0A]       ENDIF
[7C0B]       ti += 1
[7C12]       state[12] = 1200
    END
  END
[7C16]   BLOCK (exit -> @7C2C)
[7C1A]     GUARD ti == 1
[7C21]     ENDIF
[7C22]     rec_01FC |= 0x2
[7C27]     OP_C3 C3 34 02 28 00
  END
[7C2C]   BLOCK (exit -> @7DF8)
[7C30]     AWAIT presentation
[7C31]     GUARD active_actor == Ulikan.talk (related 40)
[7C36]     GUARD ti == 1
[7C3D]     ENDIF
[7C3E]     SAY ".2"
[7C48]     SAY ".1"
[7C52]     SAY ".0"
[7C5C]     SAY "CONNECT ... COMMANDER BLOOD GAME"
[7C6E]     SAY "Message: CYBERION JUNIOR HERE ."
[7C80]     SAY "MAKING PROGRESS ? ..."
[7C90]     SAY "YOU FOUND EDEN AND TINA BURNER ? ... YOU HAVE TO TELEPORT HER ..."
[7CB4]     SAY "TINA IS SOME COOL BABE ..."
[7CC8]     SAY "THIS IS COSTING ME A FORTUNE ... BYE ... ... ... ... SEE YA"
[7CEC]     IF-BLOCK (exit -> @7D23)
[7CEF]       OP_CA CA F1 C1 08 00
[7CF4]       OP_CA CA F2 C1 03 00
[7CF9]       ENDIF
[7CFA]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[7D20]       GOTO @7DDA
    END
[7D23]     IF-BLOCK (exit -> @7D46)
[7D26]       OP_CA CA F1 C1 0C 00
[7D2B]       OP_CA CA F2 C1 08 00
[7D30]       ENDIF
[7D31]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[7D43]       GOTO @7DDA
    END
[7D46]     IF-BLOCK (exit -> @7D79)
[7D49]       OP_CA CA F2 C1 15 00
[7D4E]       OP_CA CA F1 C1 04 00
[7D53]       ENDIF
[7D54]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[7D76]       GOTO @7DDA
    END
[7D79]     IF-BLOCK (exit -> @7DA4)
[7D7C]       OP_CA CA F2 C1 12 00
[7D81]       OP_CA CA F1 C1 15 00
[7D86]       ENDIF
[7D87]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[7DA1]       GOTO @7DDA
    END
[7DA4]     IF-BLOCK (exit -> @7DDA)
[7DA7]       OP_CA CA F2 C1 0E 00
[7DAC]       OP_CA CA F1 C1 12 00
[7DB1]       ENDIF
[7DB2]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[7DDA]     SAY "stop"  '[skip 4]
[7DE4]     rec_01FC &= !0x2
[7DEA]     POKE [0x7C17] = 0
[7DEE]     trak1 = 1
[7DF5]     END PRESENTATION Ulikan.talk
  END
[7DF8]   BLOCK (exit -> @7E0E)
[7DFC]     GUARD ti == 2
[7E03]     ENDIF
[7E04]     rec_01FC |= 0x2
[7E09]     OP_C3 C3 34 02 28 00
  END
[7E0E]   BLOCK (exit -> @7FB4)
[7E12]     AWAIT presentation
[7E13]     GUARD active_actor == Ulikan.talk (related 40)
[7E18]     GUARD ti == 2
[7E1F]     ENDIF
[7E20]     SAY "Network..."
[7E2A]     SAY "modem activated"
[7E36]     SAY "Rate 24,000 baud ..."
[7E46]     SAY "Connect"
[7E50]     SAY ".3"
[7E5A]     SAY ".2"
[7E64]     SAY ".1"
[7E6E]     SAY ".0"
[7E78]     SAY "CONNECT COMMANDER BLOOD GAME"
[7E88]     SAY "PIRATE MESSAGE FROM "CYBERION JUNIOR" TO OTHER GUY ..."
[7EA2]     SAY "MAXXON has been kidnapped ..."
[7EB4]     SAY "I think EVISCERATOR did it ..."
[7EC8]     IF-BLOCK (exit -> @7EFA)
[7ECB]       OP_CA CA F1 C1 08 00
[7ED0]       ENDIF
[7ED1]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[7EF7]       GOTO @7F9D
    END
[7EFA]     IF-BLOCK (exit -> @7F18)
[7EFD]       OP_CA CA F1 C1 0C 00
[7F02]       ENDIF
[7F03]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[7F15]       GOTO @7F9D
    END
[7F18]     IF-BLOCK (exit -> @7F46)
[7F1B]       OP_CA CA F2 C1 15 00
[7F20]       ENDIF
[7F21]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[7F43]       GOTO @7F9D
    END
[7F46]     IF-BLOCK (exit -> @7F6C)
[7F49]       OP_CA CA F2 C1 12 00
[7F4E]       ENDIF
[7F4F]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[7F69]       GOTO @7F9D
    END
[7F6C]     IF-BLOCK (exit -> @7F9D)
[7F6F]       OP_CA CA F2 C1 0C 00
[7F74]       ENDIF
[7F75]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[7F9D]     SAY "stop"  '[skip 3]
[7FA7]     rec_01FC &= !0x2
[7FAD]     POKE [0x7DF9] = 0
[7FB1]     END PRESENTATION Ulikan.talk
  END
[7FB4]   BLOCK (exit -> @7FCA)
[7FB8]     GUARD ti == 3
[7FBF]     ENDIF
[7FC0]     rec_01FC |= 0x2
[7FC5]     OP_C3 C3 34 02 28 00
  END
[7FCA]   BLOCK (exit -> @8136)
[7FCE]     AWAIT presentation
[7FCF]     GUARD active_actor == Ulikan.talk (related 40)
[7FD4]     GUARD ti == 3
[7FDB]     ENDIF
[7FDC]     SAY "Network..."
[7FE6]     SAY "modem activated"
[7FF2]     SAY "rate 64,000 baud..."
[8000]     SAY "Connect"
[800A]     SAY ".2"
[8014]     SAY ".1"
[801E]     SAY ".0"
[8028]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[8038]     SAY "Wha!!! That Cyberquizz exam's real tough..."
[804C]     IF-BLOCK (exit -> @807C)
[804F]       OP_CA CA F1 C1 08 00
[8054]       ENDIF
[8055]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA"  '[voice 17]
[8079]       GOTO @811F
    END
[807C]     IF-BLOCK (exit -> @809A)
[807F]       OP_CA CA F1 C1 0C 00
[8084]       ENDIF
[8085]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[8097]       GOTO @811F
    END
[809A]     IF-BLOCK (exit -> @80C8)
[809D]       OP_CA CA F2 C1 15 00
[80A2]       ENDIF
[80A3]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[80C5]       GOTO @811F
    END
[80C8]     IF-BLOCK (exit -> @80EE)
[80CB]       OP_CA CA F2 C1 12 00
[80D0]       ENDIF
[80D1]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[80EB]       GOTO @811F
    END
[80EE]     IF-BLOCK (exit -> @811F)
[80F1]       OP_CA CA F2 C1 0C 00
[80F6]       ENDIF
[80F7]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[811F]     SAY "stop"  '[skip 3]
[8129]     rec_01FC &= !0x2
[812F]     POKE [0x7FB5] = 0
[8133]     END PRESENTATION Ulikan.talk
  END
[8136]   BLOCK (exit -> @814C)
[813A]     GUARD ti == 4
[8141]     ENDIF
[8142]     rec_01FC |= 0x2
[8147]     OP_C3 C3 34 02 28 00
  END
[814C]   BLOCK (exit -> @82DC)
[8150]     AWAIT presentation
[8151]     GUARD active_actor == Ulikan.talk (related 40)
[8156]     GUARD ti == 4
[815D]     ENDIF
[815E]     SAY "Network..."
[8168]     SAY "modem activated"
[8174]     SAY "rate 280,000 baud..."
[8182]     SAY "Connect"
[818C]     SAY ".5"
[8196]     SAY ".4"
[81A0]     SAY ".3"
[81AA]     SAY ".2"
[81B4]     SAY ".1"
[81BE]     SAY ".0"
[81C8]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[81D8]     SAY "I teleported Bronko to ERAZOR ..."
[81EC]     IF-BLOCK (exit -> @8226)
[81EF]       OP_CA CA F1 C1 08 00
[81F4]       ENDIF
[81F5]       SAY "EITHER YOU GET UP EARLY TO PLAY OR YOU DIDN'T GO TO BED... GOTTA GO ... SEE YA !"  '[voice 17]
[8223]       GOTO @82C5
    END
[8226]     IF-BLOCK (exit -> @8246)
[8229]       OP_CA CA F1 C1 0C 00
[822E]       ENDIF
[822F]       SAY "BEAUTIFUL MORNING ... SEE YA ..."  '[voice 16]
[8243]       GOTO @82C5
    END
[8246]     IF-BLOCK (exit -> @8274)
[8249]       OP_CA CA F2 C1 15 00
[824E]       ENDIF
[824F]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[8271]       GOTO @82C5
    END
[8274]     IF-BLOCK (exit -> @829A)
[8277]       OP_CA CA F2 C1 12 00
[827C]       ENDIF
[827D]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[8297]       GOTO @82C5
    END
[829A]     IF-BLOCK (exit -> @82C5)
[829D]       OP_CA CA F2 C1 0C 00
[82A2]       ENDIF
[82A3]       SAY "I'M GOING TO DIVE IN THE POOL ... IT'S A SUNNY AFTERNOON ..."  '[voice 13]
    END
[82C5]     SAY "stop"  '[skip 3]
[82CF]     rec_01FC &= !0x2
[82D5]     POKE [0x8137] = 0
[82D9]     END PRESENTATION Ulikan.talk
  END
[82DC]   BLOCK (exit -> @82F2)
[82E0]     GUARD ti == 5
[82E7]     ENDIF
[82E8]     rec_01FC |= 0x2
[82ED]     OP_C3 C3 34 02 28 00
  END
[82F2]   BLOCK (exit -> @84BE)
[82F6]     AWAIT presentation
[82F7]     GUARD active_actor == Ulikan.talk (related 40)
[82FC]     GUARD ti == 5
[8303]     ENDIF
[8304]     SAY "Network ..."
[8310]     SAY "modem activated"
[831C]     SAY "rate 240,000 baud..."
[832A]     SAY "Connect"
[8334]     SAY ".5"
[833E]     SAY ".4"
[8348]     SAY ".3"
[8352]     SAY ".2"
[835C]     SAY ".1"
[8366]     SAY ".0"
[8370]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[8380]     SAY "Wowee !!! Beauregard and his mummies ... Excellent !!! ..."
[839C]     IF-BLOCK (exit -> @83B4)
[839F]       OP_CB CB F5 19 0C CA 07
[83A5]       ENDIF
[83A6]       SAY "HAPPY CHRISTMAS ..."
    END
[83B4]     IF-BLOCK (exit -> @83CE)
[83B7]       OP_CB CB F5 01 01 CB 07
[83BD]       ENDIF
[83BE]       SAY "HAPPY NEW YEAR ..."
    END
[83CE]     IF-BLOCK (exit -> @83FE)
[83D1]       OP_CA CA F1 C1 08 00
[83D6]       ENDIF
[83D7]       SAY "YOU SURE GET UP EARLY TO PLAY ... GOTTA GO ... SEE YA !"  '[voice 17]
[83FB]       GOTO @84A7
    END
[83FE]     IF-BLOCK (exit -> @841E)
[8401]       OP_CA CA F1 C1 0C 00
[8406]       ENDIF
[8407]       SAY "BEAUTIFUL MORNING ... SEE YA !"  '[voice 16]
[841B]       GOTO @84A7
    END
[841E]     IF-BLOCK (exit -> @844C)
[8421]       OP_CA CA F2 C1 15 00
[8426]       ENDIF
[8427]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[8449]       GOTO @84A7
    END
[844C]     IF-BLOCK (exit -> @8472)
[844F]       OP_CA CA F2 C1 12 00
[8454]       ENDIF
[8455]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[846F]       GOTO @84A7
    END
[8472]     IF-BLOCK (exit -> @84A7)
[8475]       OP_CA CA F2 C1 0C 00
[847A]       ENDIF
[847B]       SAY "IT'S A BEAUTIFUL AFTERNOON ... THINK I'LL TAKE MY DOG FOR A WALK ... COME ON BOY !"  '[voice 13]
    END
[84A7]     SAY "stop"  '[skip 3]
[84B1]     rec_01FC &= !0x2
[84B7]     POKE [0x82DD] = 0
[84BB]     END PRESENTATION Ulikan.talk
  END
[84BE]   BLOCK (exit -> @855E)
[84C2]     AWAIT gameflag_274F
[84C3]     GUARD active_actor == Bob_Morlock.talk (related 40)
[84C8]     ENDIF
[84C9]     SAY "..."  '[skip 1]
[84D3]     state[1] = 30
[84D7]     IF-BLOCK (exit -> @8514)
[84DA]       GUARD state[1] == 0
[84DC]       ENDIF
[84DD]       SAY "I don't feel at all well , Commander ..."  '[voice 6]
[84F7]       SAY "Ahhhh !!!"  '[voice 3]
[8503]       SAY "stop"  '[skip 2]
[850D]       state[1] = 65535
[8511]       END PRESENTATION Bob_Morlock.talk
    END
[8514]     SAY "I feel so weak ... word_65535 bye_bye"  '[voice 6, skip 1]
[852A]     adieu = 1
[8531]     IF-BLOCK (exit -> @855E)
[8534]       GUARD adieu == 1
[853B]       ENDIF
[853C]       SAY "..."
[8546]       SAY "stop"  '[skip 3]
[8550]       adieu = 0
[8557]       state[1] = 65535
[855B]       END PRESENTATION Bob_Morlock.talk
    END
  END
[855E]   BLOCK (exit -> @8660)
[8562]     AWAIT gameflag_274F
[8563]     GUARD active_actor == Bob_Morlock.talk (related 40)
[8568]     GUARD emplo == 1
[856F]     GUARD emp == 0
[8576]     ENDIF
[8577]     SAY "By the way , Cap'n Bob sir , there's something I wanted to talk to you about ..."
[85A3]     SAY "Ginette and I would like a raise ... Even a few megawatts would be appreciated ..."
[85CB]     SAY "WHAT ? SPEAK LOUDER ... I'M GETTING A LITTLE DEAF COMMANDER ..."  '[voice 5]
[85EB]     SAY "WE WANT A RAISE !!!"
[85FD]     SAY "I don't feel at all well , Commander . Aaah !!! It's my heart ...."  '[voice 4]
[8623]     SAY "Heart ... He means his generator ... They're all the same , as soon as you mention a couple of lousy megawatts ..."  '[skip 1]
[8659]     emp = 1
  END
[8660]   BLOCK (exit -> @8839)
[8664]     AWAIT gameflag_274F
[8665]     GUARD active_actor == Bob_Morlock.talk (related 40)
[866A]     GUARD reve == 1
[8671]     GUARD revelat == 0
[8678]     ENDIF
[8679]     SAY "You want to know an unbearable truth, Commander ?"  '[voice 7]
[8693]     SAY "HONK ! Switch yourself off for ten seconds !!!"  '[voice 6]
[86AD]     SAY "Bu ... But Cap'n Bob ... I ..."
[86C5]     SAY "SWITCH OFF I SAID !!!"  '[voice 5]
[86D7]     SAY "Okay sir ....."
[86E5]     SAY "KRUIIIIK !!! AAAaaaaaaaaaaaaaaaaaaaaa !!!"
[86F5]     SAY "COMMANDER, YOU ARE ME ...."  '[voice 5]
[8707]     SAY "WE ARE THE SAME BEING AT TWO DIFFERENT AGES ..."  '[voice 6]
[8723]     SAY "YOU ARE MUCH MORE THAN A SON TO ME ..."  '[voice 4]
[873F]     SAY "We're the same person ... I am the first being to create himself"  '[voice 5]
[8761]     SAY "And who can study himself when he was younger , all thanks to space and time . YOU ARE BOB , COMMANDER ..."  '[voice 4]
[8797]     SAY "I am who you'll become in a few hundred thousand years ..."  '[voice 2]
[87B7]     SAY "OK Honk , We've finished ..."  '[voice 6]
[87CB]     SAY "KROIIIIkkk !!! -&KRUIIIIkkk !!! I ought to be aware of everything that goes on here . I AM the onboard computer, aren't I ? ... You didn't switch off Olga I hope"
[8813]     SAY "Can it and get to work !!! ..."  '[voice 5, skip 2]
[882B]     revelat = 1
[8832]     reve = 0
  END
[8839] END OF SCRIPT
