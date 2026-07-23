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
[00CD]     GUARD active_actor == rec_081C (related 40)
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
[0403]     END PRESENTATION rec_081C
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
[04A1]       END PRESENTATION rec_0504
    END
  END
[04A4]   BLOCK (exit -> @04D6)
[04A8]     AWAIT gameflag_274F
[04A9]     GUARD active_actor == rec_0504 (related 40)
[04AE]     ENDIF
[04AF]     SAY "Commander , this is one tough repair job ..."
[04C9]     SAY "stop"  '[skip 1]
[04D3]     END PRESENTATION rec_0504
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
[04F7]     GUARD active_actor == rec_06B4 (related 40)
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
[05D8]     END PRESENTATION rec_06B4
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
[0604]     GUARD active_actor == rec_0624 (related 40)
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
[06F3]     END PRESENTATION rec_0624
  END
[06F6]   BLOCK (exit -> @0881)
[06FA]     AWAIT gameflag_252A
[06FB]     GUARD active_actor == rec_0624 (related 40)
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
[087E]     END PRESENTATION rec_0624
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
[08AC]   START PRESENTATION rec_0624 (related 40)
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
[09A6]   END PRESENTATION rec_0624
[09A9]   BLOCK (exit -> @0A6B)
[09AD]     AWAIT gameflag_252A
[09AE]     GUARD active_actor == rec_0624 (related 40)
[09B3]     GUARD panne == 0
[09BA]     ENDIF
[09BB]     SAY "Ha! Hi guy . I was wondering when you'd show up ..."  '[voice 1]
[09DB]     SAY "My engine's totalled ... I must've pushed it too hard ... It just went : ARG ... PSHHHHHH !"  '[voice 2]
[0A09]     SAY "Think you can fix it for me ?"  '[voice 3]
[0A21]     SAY "Teleport Morning Oil over to him , Commander . He's an ace repairman ..."
[0A45]     SAY "I'm waiting , guy ..."  '[voice 4]
[0A57]     SAY "..."  '[skip 2]
[0A61]     panne = 1
[0A68]     END PRESENTATION rec_0624
  END
[0A6B]   BLOCK (exit -> @0C75)
[0A6F]     AWAIT gameflag_252A
[0A70]     GUARD active_actor == rec_0624 (related 40)
[0A75]     ENDIF
[0A76]     IF-BLOCK (exit -> @0ACF)
[0A79]       GUARD panne == 1
[0A80]       GUARD rec_03C2 == 4246
[0A85]       ENDIF
[0A86]       SAY "I'm still waiting , old buddy . You're not gonna let me down now , huh ? ..."  '[voice 1]
[0AB2]       SAY "Commander , Morning Oil is an expert repairman ..."  '[skip 1]
[0ACC]       END PRESENTATION rec_0624
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
[0C72]     END PRESENTATION rec_0624
  END
[0C75]   BLOCK (exit -> @0D76)
[0C79]     AWAIT gameflag_252A
[0C7A]     GUARD rec_03C2 == 4246
[0C7F]     GUARD B1 == 0
[0C86]     GUARD active_actor == rec_03E4 (related 40)
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
[0D3F]         END PRESENTATION rec_03E4
      END
[0D42]       IF-BLOCK (exit -> @0D69)
[0D45]         GUARD concept == "refuse"
[0D48]         ENDIF
[0D49]         SAY "OK , Commander . Your wish is my command ..."  '[skip 2]
[0D65]         CLEAR concept_alt
[0D66]         END PRESENTATION rec_03E4
      END
    END
[0D69]     SAY "..."  '[skip 1]
[0D73]     END PRESENTATION rec_03E4
  END
[0D76]   BLOCK (exit -> @0E7A)
[0D7A]     AWAIT gameflag_274F
[0D7B]     GUARD active_actor == rec_03E4 (related 40)
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
[0E2E]       END PRESENTATION rec_03E4
    END
[0E31]     SAY "I hear and obey , Commander ..."
[0E47]     SAY "Don't you love the way I programmed him ... Total docility is his watchword ..."
[0E6D]     SAY "..."  '[skip 1]
[0E77]     END PRESENTATION rec_03E4
  END
[0E7A]   BLOCK (exit -> @1141)
[0E7E]     AWAIT gameflag_274F
[0E7F]     GUARD NOT bronk4 == 1082
[0E85]     GUARD F1 == 0
[0E8C]     GUARD NOT rec_1088 == 2684
[0E92]     GUARD active_actor == rec_0474 (related 40)
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
[113E]     END PRESENTATION rec_0474
  END
[1141]   BLOCK (exit -> @12EF)
[1145]     AWAIT gameflag_274F
[1146]     GUARD rec_1088 == 2684
[114B]     GUARD NOT bronk4 == 1082
[1151]     GUARD active_actor == rec_0474 (related 40)
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
[12C3]         END PRESENTATION rec_0474
      END
[12C6]       IF-BLOCK (exit -> @12E2)
[12C9]         GUARD concept == "NO"
[12CC]         ENDIF
[12CD]         SAY "As you wish , Commander ..."  '[skip 1]
[12E1]         CLEAR concept_alt
      END
    END
[12E2]     SAY "..."  '[skip 1]
[12EC]     END PRESENTATION rec_0474
  END
[12EF]   BLOCK (exit -> @13A9)
[12F3]     AWAIT gameflag_252A
[12F4]     GUARD rec_1088 == 2684
[12F9]     GUARD active_actor == rec_0474 (related 40)
[12FE]     ENDIF
[12FF]     SAY "Commander , There's nobody here ..."  '[voice 5]
[1313]     SAY "I'll look around , Commander ... Use the phone to call me ..."  '[voice 2]
[1335]     SAY "Good luck , Mister Bronko . I admire the way you get things done ..."
[135B]     SAY "Shucks , Mister Honk ... It just the way nature made me ..."  '[voice 6]
[137D]     SAY "Nature . What a wonderful invention ..."
[1393]     SAY "..."  '[skip 3]
[139D]     rec_043C |= 0x2
[13A2]     POKE [0x12F0] = 0
[13A6]     END PRESENTATION rec_0474
  END
[13A9]   BLOCK (exit -> @15F5)
[13AD]     AWAIT presentation
[13AE]     GUARD rec_0452 == 2684
[13B3]     GUARD active_actor == rec_0474 (related 40)
[13B8]     ENDIF
[13B9]     IF-BLOCK (exit -> @142E)
[13BC]       GUARD NOT rec_1088 == 2684
[13C2]       ENDIF
[13C3]       SAY "Krrrr Bzzz This is Bronko .... Bzzzzz I can't hear you .... Krrrk ..."
[13E7]       SAY "kkkkKrouik..."
[13F1]       SAY "He's having auditory difficulties , Commander . He's too far away ... We ought to get closer to planet Erazor..."
[1421]       SAY "..."  '[skip 1]
[142B]       END PRESENTATION rec_0474
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
[15F2]       END PRESENTATION rec_0474
    END
  END
[15F5]   BLOCK (exit -> @1723)
[15F9]     AWAIT gameflag_252A
[15FA]     GUARD rec_1088 == 2684
[15FF]     GUARD rec_0452 == 2684
[1604]     GUARD active_actor == rec_0474 (related 40)
[1609]     ENDIF
[160A]     IF-BLOCK (exit -> @167B)
[160D]       GUARD NOT bronk4 == 1082
[1613]       ENDIF
[1614]       SAY "What are you doing , Commander ? ... You're going to get us spotted ..."  '[voice 1]
[163A]       SAY "Use the phone to contact me ..."  '[voice 2]
[1650]       SAY "He's right , Commander . You'll get him spotted ! ..."
[166E]       SAY "..."  '[skip 1]
[1678]       END PRESENTATION rec_0474
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
[1720]         END PRESENTATION rec_0474
      END
    END
  END
[1723]   BLOCK (exit -> @1879)
[1727]     AWAIT gameflag_274F
[1728]     GUARD bronk4 == 1082
[172D]     GUARD active_actor == rec_0474 (related 40)
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
[17BE]       END PRESENTATION rec_0474
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
[1876]     END PRESENTATION rec_0474
  END
[1879]   GOTO @1936
[187D]   AWAIT gameflag_274F
[187E]   START PRESENTATION rec_0474 (related 40)
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
[1933]   END PRESENTATION rec_0474
[1936]   GOTO @1973
[193A]   AWAIT gameflag_274F
[193B]   START PRESENTATION rec_027C (related 40)
[1940]   state[2] = 43695
[1944] ?? invalid opcode 13
