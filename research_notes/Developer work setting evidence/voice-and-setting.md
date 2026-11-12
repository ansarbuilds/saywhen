# Voice input and where developers work

Scope note for the report writer: saywhen is a desktop app that listens with a webcam and microphone, with no hotkey, while the developer looks at the screen, then pastes text into the focused app. It is a poor fit for a quiet open office. These notes do not invent a market for it. Anecdotes are named and are not turned into rates. Disability prevalence is not voice-coding adoption. Office-worker surveys are not relabeled as developers.

Date of research: 12 November 2026.

## Do developer surveys measure voice input, dictation, speech-to-text, or voice coding?

### Takeaway

No Stack Overflow, JetBrains, or GitHub survey result was found that reports a share of developers who use voice input, dictation, speech-to-text, or voice coding. Academic papers that were opened are small user studies of people asked to speak code, or of people already recruited because they voice-code. That evidence is anecdotal or experimental. It is not an adoption rate.

### Cited Findings

- No usable adoption statistic. Searches of Stack Overflow Developer Survey pages, JetBrains Developer Ecosystem pages, and GitHub Octoverse coverage did not return a question such as "do you use voice input / dictation / speech-to-text while coding" with a percentage and a sample. — [Stack Overflow survey search results](https://survey.stackoverflow.co/2025); [JetBrains Developer Ecosystem 2023](https://www.jetbrains.com/lp/devecosystem-2023); pages checked did not contain this question.

- Blog roundups that cite Stack Overflow and JetBrains are about AI coding tools, not voice coding. They were not used. Examples checked: [FreqDeck, "AI Coding and Voice Statistics 2026"](https://freqdeck.dev/guides/ai-voice-coding-statistics-2026/) and [VoxBooster, "AI Coding Tools Statistics 2026"](https://voxbooster.com/blog/ai-coding-tools-statistics-2026). Neither excerpt reported a survey share of developers who code by voice. Do not convert their AI-tool percentages into a voice-input market.

- Creed and Sarcar, voice-coding interviews, author-accepted manuscript dated in the repository as 2024. This is not a prevalence survey. The sample was recruited because the people already voice-code.
  - Claim: Experienced voice coders with physical impairments were interviewed about their practice; most of this recruited group used Talon.
  - Number: 7 interview participants; 6 of those 7 used Talon; 1 had previously used Dragon NaturallySpeaking; 5 of the 7 later tried a prototype. Do not report 6/7 as the share of developers who use Talon.
  - Year: 2024 manuscript.
  - Population: voice coders with physical impairments, ages 20 to 41, coding experience 2.5 to 27 years. Not a sample of developers in general.
  - Geography: not stated as a national sample. Interviews were remote.
  - Source name: Chris Creed and Sayan Sarcar, "Voice Coding Experiences for Developers with Physical Impairments," HCI Research Centre, Birmingham City University.
  - URL: https://www.open-access.bcu.ac.uk/15942/1/Creed_Voice_Coding_Experiences_Disabled_Developers%20%28Author%20Accepted%20Manuscript%29.pdf
  - Sample size: 7 interviews, then 5 in the follow-up evaluation.
  - Caveat: Everyone was recruited for voice-coding experience. A count inside that group is not adoption.
  - Quoted question: the paper describes semi-structured interviews about tools, impairments, and team work. The exact questionnaire items were not printed in the extracted text. The paper's own summary: "Semi-structured interviews were initially conducted with seven voice coders with physical impairments."

- Nowrin and Vertanen, CUI 2023. This is a remote user study of how people speak code to a hypothetical system, not a survey of how many developers use voice coding.
  - Claim: In a 24-person speaking study, a small minority agreed they frequently used speech interfaces. The paper does not say they voice-code at work.
  - Number: 8% of novices and 18% of experts "strongly agreed or agreed that they frequently used speech interfaces." With 12 people in each group, these percentages are one or two people. Do not generalize them.
  - Year: 2023 (CUI '23, 19-21 July 2023, Eindhoven).
  - Population: 12 novice programmers from introductory Java courses and 12 expert programmers with at least four years of experience, recruited by word of mouth. All were native English speakers.
  - Geography: not a national sample. The conference was in the Netherlands; recruitment geography was not stated in the extract.
  - Source name: Sadia Nowrin and Keith Vertanen, "Programming by Voice: Exploring User Preferences and Speaking Styles."
  - URL: https://www.keithv.com/pub/progvoice/nowrin_progvoice.pdf and https://dl.acm.org/doi/fullHtml/10.1145/3571884.3597130
  - Sample size: 24 (12 novice, 12 expert).
  - Caveat: "Speech interfaces" is not "voice coding while working." The exact Likert item is in supplementary materials that were not opened. Participants were paid or given course credit to speak code for the study.
  - Quoted question: not printed in the paper body that was retrieved. The paper's paraphrase: "As for their usage of speech interfaces, 8% of novices and 18% of experts strongly agreed or agreed that they frequently used speech interfaces."

- Begel and Graham, 2006, SPEED usability study. Anecdotal. Five expert Java programmers. Not a rate.
  - Claim: In this lab study, most participants had never used speech recognition; those who had, had abandoned it because of poor accuracy. The participants said they would not use this environment for daily coding. One said he might use voice at home if he worked alone, because he would not bother anyone else. Participants also named noise, the interference of speaking with thinking, and poor recognizer accuracy.
  - Number: no population percentage. Session 1 had 3 people (Dragon NaturallySpeaking). Session 2 had 2 people (a human typed what they said).
  - Year: 2006.
  - Population: expert Java programmers familiar with Eclipse, average 12 years of experience, from the software industry. None had a motor disability that made typing difficult.
  - Geography: not stated.
  - Source name: Andrew Begel, "An Assessment of a Speech-Based Programming Environment" (SPEED; Microsoft Research author copy).
  - URL: https://andrewbegel.com/papers/speed-short-paper.pdf
  - Sample size: 5.
  - Caveat: 2006 recognizer quality. Comments are from the five participants, not a survey. Do not turn "none of them would use this for daily coding" into "0% of developers want voice coding."
  - Quoted question: the paper says they were asked whether they would consider using voice recognition for programming. The verbatim item was not printed. The paper's result sentence: "The participants concluded that none of them would use this programming environment for daily coding, especially with the poor accuracy provided by the machine voice recognizer."

### Inferences

- A report can say that voice coding shows up in small accessibility and HCI studies, and in tools such as Talon, and that those studies do not measure how common the practice is.
- A report cannot say that X% of developers use voice input, or that use is rare at a specific rate, or that use is growing at a specific rate. "Not found" is the survey answer.
- The Begel comments about noise and working alone are relevant to a hands-free, always-available mic, but they are five people's reactions to a 2006 lab system.

### Gaps

- JetBrains Developer Ecosystem: no voice, dictation, or speech-to-text adoption question was found. The 2023 report's "49% of respondents engage in remote development" is about remote and collaborative development tools, on a page section titled "Remote and collaborative development," not about working from home and not about voice. Do not reuse 49%. Source: https://www.jetbrains.com/lp/devecosystem-2023 (26,348 developers in that year's report, global).
- GitHub Octoverse: no voice-coding survey question was found.
- A population survey of developers who want voice input: not found. See the later section.
- Maloku and Pllana's HyperCode user study is cited by Nowrin as a timing comparison of keyboard, mouse, and voice. That paper was not opened, so no time or percentage from it is reported here.

## Do accessibility surveys count voice input while working?

### Takeaway

No accessibility survey was found that states a number for developers using speech recognition while working. WebAIM's screen-reader surveys ask about screen readers. "VoiceOver" in those surveys is Apple's screen reader, not dictation. WebAIM's only speech-recognition percentages found here are from a 46-person 2013 survey of motor-disabled web users. That is not voice coding and must not be used as adoption.

### Cited Findings

- Stack Overflow disability items that were retrieved do not ask about voice input while working.
  - 2022 overview shows the question "Which of the following describe you, if any? Please check all that apply" under disability status, and says "2,547 respondents have a physical difference." The retrieved text did not list speech recognition, dictation, or voice coding as a response. — [Stack Overflow Developer Survey 2022](https://survey.stackoverflow.co/2022/)
  - 2017 overview says this was an early disability question, confined to disabilities that need a physical accommodation, and lists "Unable to type" at 0.1% next to other impairment options. "Unable to type" is not use of voice input. The denominator for 0.1% was not clear in the extract, so that figure is not used. — [Stack Overflow Developer Survey 2017](https://survey.stackoverflow.co/2017)
  - The 2026 question-list preview that was retrieved includes "Which best describes your current work situation?" and many tool questions. No voice, dictation, speech-to-text, or microphone question appeared in the retrieved text. This is not a certified line-by-line audit of every hidden item. — [Stack Overflow 2026 survey question preview](https://questions.survey.stackoverflow.co/)

- WebAIM Screen Reader User Surveys #8 through #11, including survey #10 (December 2023-January 2024, 1,539 responses) and survey #11 (results page updated 29 September 2026), report primary screen readers such as JAWS, NVDA, and VoiceOver. The retrieved results do not report a share who use speech recognition to program. VoiceOver percentages are screen-reader use. — [Survey #10](https://webaim.org/projects/screenreadersurvey10/); [Survey #11](https://webaim.org/projects/screenreadersurvey11/)

- Rejected figure, included only so it is not mistaken for developer adoption. WebAIM Survey of Users with Motor Disabilities, March 2013.
  - Claim: among a very small self-selected group of web users with motor disabilities, some used speech recognition to interact with web pages.
  - Number: "Using speech recognition" as a method of interaction with web pages: 6 respondents, 13.3%. "Speech recognition" as a technology used: 14 respondents, 30.4%. These are different questions. Neither is voice coding.
  - Year: 2013. Last updated on the page: 29 April 2013.
  - Population: users with motor disabilities who answered a WebAIM web survey. Not developers. Not people at work coding.
  - Geography: not stated in the extract.
  - Source name: WebAIM, "Survey of Users with Motor Disabilities."
  - URL: https://webaim.org/projects/motordisabilitysurvey/
  - Sample size: 46 valid responses. WebAIM says the sample is very small and "should not be used as an indication of the prevalence of these disabilities generally."
  - Caveat: do not cite 13.3% or 30.4% as the share of developers who use voice input.
  - Quoted questions, from the result tables: "Methods of Interaction with Web Pages" includes "Using speech recognition"; "Technologies Used" includes "Speech recognition."

### Inferences

- There is no survey evidence in these sources for how many working developers dictate code or commands.
- Accessibility need (motor disability, inability to type) is a different construct from product adoption.

### Gaps

- A current WebAIM or SO statistic of the form "X% of developers use speech recognition at work" was not found.
- If a later draft needs a disability-prevalence number, it still must not be used as a voice-coding number. No such prevalence number is supplied here on purpose.

## What share of developers work alone, from home, or in a private room?

### Takeaway

Developer surveys measure remote, hybrid, and in-person work. They do not ask whether the person is alone, in a private room, or in an open office. In the 2026 Stack Overflow Developer Survey, 34.8% of people who answered the work-setting question said remote, and 17.6% said in-person. The rest were hybrid or "your choice." Working from home is not the same as having a private room where talking is acceptable. "Company of one" is an employment category, not a room.

### Cited Findings

- Stack Overflow Developer Survey 2026, work setting. This is the cleanest current developer figure.
  - Claim: among respondents who described their current work situation, remote was the single largest category, and fully in-person was under one in five.
  - Number: Remote 8,564 (34.8%); Hybrid, some remote, leans heavy to in-person 4,883 (19.9%); In-person 4,335 (17.6%); Hybrid, some in-person, leans heavy to flexibility 3,823 (15.5%); Your choice, very flexible, you can come in when you want or just as needed 2,997 (12.2%). Counts sum to 24,602.
  - Year: 2026. Press release dated 6 October 2026.
  - Population: Stack Overflow Developer Survey respondents who answered this question. The press release describes the whole survey as more than 30,000 responses from technologists in 169 countries. Not a random sample of all developers. Not filtered to people who sit in a private room.
  - Geography: global. The data page's analysis text says remote work is 48% in the USA (n = 3,940 for the USA cut) and 21% in France (n = 679), and that India has the largest in-person share at 28% (India n = 951). Germany n = 1,746 and UK n = 1,175 are the other cuts listed. The full country percentage table was not extracted beyond those sentences.
  - Source name: Stack Exchange Inc., "Work Setting," Stack Overflow Developer Survey 2026.
  - URL: https://survey.stackoverflow.co/2026/work/data/work-setting
  - Sample size: 24,602 for the all-respondent table.
  - Caveat: self-selected survey. "Remote" can be a home, a coworking space, or somewhere else. The question does not ask "are you alone?" or "are you in a private room?" Hybrid and "your choice" include people who are in an office on some or many days. Do not add the non-in-person categories and call the sum "working alone."
  - Quoted question: "Which best describes your current work situation?"

- Same 2026 survey, employment form, different question. Do not treat this as solitude.
  - Claim: the share saying they were a company of one rose sharply.
  - Number: the survey work narrative says the share "saying it was just them in an org jumped from 4% to 10%." The Stack Overflow press release says freelancers and sole proprietors "account for 11% of respondents, up from 4% in 2025." The two Stack Overflow pages disagree by about one point on the 2026 figure (10% vs 11%).
  - Year: 2026, compared with 2025.
  - Population: Developer Survey respondents. Press-release denominator is the survey dataset; the work-page denominator is not separately printed in the extract.
  - Geography: global survey.
  - Source name: Stack Overflow Developer Survey 2026 work chapter; Stack Overflow press release, 6 October 2026.
  - URLs: https://survey.stackoverflow.co/2026/work and https://stackoverflow.co/company/press/archive/stack-overflow-annual-developer-survey-ai-trust-conditional
  - Sample size: not stated for this specific item in the extracted text. Whole survey: more than 30,000 responses.
  - Caveat: this is firm size / self-employment, not a private room and not "working with the microphone on." Report the 10% vs 11% conflict rather than picking one.
  - Quoted question: not printed as a single stem in the extract. The work page glosses it as people "saying it was just them in an org" and as "freelancers and sole proprietors."

- Stack Overflow Developer Survey 2025, work environment. The five-way percentages below are from Stack Overflow's own chart caption on a 30 September 2026 retrospective, because the 2025 survey HTML chart did not come through as text. The response count and the US and Germany sentences are from the survey page itself.
  - Claim: in 2025, fully remote was about a third of answers, and strict in-person was under a fifth.
  - Number: Remote 32.4%; Hybrid (some remote) 19.9%; In-person 17.9%; Hybrid (some in-person) 17.2%; Your choice 12.6%. These five sum to 100%. Stack Overflow's retrospective also says 82% had "some degree of remote work flexibility" in 2025, which is everyone except the 17.9% in-person group, rounded. Do not describe 82% as fully remote.
  - Year: 2025 survey, described by Stack Overflow on 30 September 2026.
  - Population: people who answered the work-environment question.
  - Geography: global. Survey page: "the US has the highest number of developers working remotely (45%). 21% of developers in Germany say the choice to go into the office or work remotely is completely up to them." The retrospective adds India on-site 30.5%, Germany fully remote 22.5%, and other country cuts; those extra country figures are the blog's, not a data file that was opened.
  - Source name: Stack Overflow Developer Survey 2025; Stack Overflow blog, "Getting ready for 2026 results."
  - URLs: https://survey.stackoverflow.co/2025/work and https://stackoverflow.blog/2026/09/30/getting-ready-for-2026-results-a-look-back-on-developer-survey-findings
  - Sample size: the survey page says Responses: 33,686 (68.7% of that year's survey). The survey homepage says over 49,000 responses from 177 countries across 62 questions.
  - Caveat: the blog caption calls the chart "work environment preferences," while the survey page discusses where people are working. The 2026 question, which uses the same five labels, asks for the current situation. Treat 2025 as current work situation unless a later look at the raw 2025 questionnaire says otherwise. Country n for the 45% and 21% lines was not on the extracted survey page.
  - Quoted question: the 2025 page extract did not include the stem. The matching 2026 stem is "Which best describes your current work situation?"

- Stack Overflow Developer Survey 2024, work environment, from the survey page's own prose, plus the retrospective for the remote share.
  - Claim: hybrid stayed at the prior year's level, and in-person rose for a third year.
  - Number: hybrid 42%; in-person 20% in 2024, compared with 16% in 2023 and 15% in 2022. The retrospective says 2024 was 38.0% remote, 42.0% hybrid, and 20.0% in person. 38 + 42 + 20 = 100, so the 38% remote figure matches the residual of the survey page's hybrid and in-person shares, but the survey page extract itself did not print "38%."
  - Year: 2024, with 2022 and 2023 comparisons stated on the 2024 page.
  - Population: Developer Survey respondents.
  - Geography: global survey. The page discusses the United States, Germany, India, the United Kingdom, and Ukraine for employment, not a private-office cut.
  - Source name: Stack Overflow Developer Survey 2024; retrospective as above.
  - URLs: https://survey.stackoverflow.co/2024/work and https://stackoverflow.blog/2026/09/30/getting-ready-for-2026-results-a-look-back-on-developer-survey-findings
  - Sample size: Responses: 54,806 (83.8%) for the work-environment block.
  - Caveat: three categories in 2024, five in 2025 and 2026, so 2024 "hybrid" is not identical to either 2025 hybrid row. Still not a private-room measure.
  - Quoted question: not in the extracted 2024 text.

- Stack Overflow's retrospective on earlier years, used only for the historical remote series, and only with the conflict noted.
  - Claim: Stack Overflow says fully remote was 12% in 2019 and peaked at 43% in 2022, and that hybrid was the majority at 42% in 2023 and 2024.
  - Number: 12% fully remote (2019); 43% fully remote (2022); hybrid 42% (2023 and 2024). The 2024 hybrid figure agrees with the 2024 survey page. The 43% figure was not visible in the 2022 overview extract.
  - Year: retrospective published 30 September 2026, describing 2019-2024 surveys.
  - Population: Developer Survey respondents in those years.
  - Geography: global.
  - Source name: Stack Overflow blog retrospective.
  - URL: https://stackoverflow.blog/2026/09/30/getting-ready-for-2026-results-a-look-back-on-developer-survey-findings
  - Sample size: not restated in the retrospective for 2019 or 2022.
  - Caveat: the 2022 survey overview that was opened says a different thing: "85% of developers say their organizations are at least partially remote." That is an organization statement, not "43% of developers are fully remote." Both can be true. Do not swap them. The 43% and 12% figures were not re-read off the 2019 or 2022 charts.
  - Quoted question: not in the retrospective. 2022 overview URL: https://survey.stackoverflow.co/2022/

- Stack Overflow Developer Survey 2017, older remote-work baseline.
  - Claim: a majority said they worked remotely at least sometimes; full-time remote was much smaller; remote options were a common job priority.
  - Number: 63.9% worked remotely at least one day a month; 11.1% full-time remote or almost all the time; 53.3% said remote options were a top priority in a new job.
  - Year: 2017.
  - Population: Developer Survey respondents.
  - Geography: global survey.
  - Source name: Stack Overflow Developer Survey 2017 overview.
  - URL: https://survey.stackoverflow.co/2017
  - Sample size: not stated for these three lines in the extract.
  - Caveat: overview paraphrase, nine years before this research. Not a private room. The verbatim question stems were not in the extract.
  - Quoted question: not retrieved. The page's wording is "A majority of developers, 63.9%, reported working remotely at least one day a month, and 11.1% say they're full-time remote or almost all the time."

- Knowledge workers, not developers. Survey of Working Arrangements and Attitudes (SWAA), WFH Research, August 2025 deck.
  - Claim: about 27% of US paid days in July 2025 were work-from-home days. By spring 2025, among full-time employees, 13% were fully remote, 60% were full-time on site, and 27% were hybrid. Office-facility workers' work-from-home rate stayed between 43% and 46% of full days from November 2024 to July 2025, and the slide title says that rate is double the rate for other workers. The other workers' percentage was not in the extracted text, so "double" is the authors' description, not a second number to invent.
  - Number: 27% of paid days (July 2025, all workers in that chart); 13% / 60% / 27% arrangement split (spring 2025, full-time wage and salary employees); 43% to 46% of full days at home for office-facility workers (November 2024-July 2025).
  - Year: data through July 2025; deck dated 5 August 2025.
  - Population: US residents ages 20-64 in the SWAA, with the earnings rules described on the slides. Office-worker cuts are people whose workplace facility is an office, or who say the office is where they would go if they are fully remote. This includes many non-developers and excludes developers whose site is not coded as an office. Do not call them developers or software engineers.
  - Geography: United States.
  - Source name: WFH Research, "SWAA August 2025 Updates" (Barrero, Bloom, Davis, and colleagues' Survey of Working Arrangements and Attitudes).
  - URL: https://wfhresearch.com/wp-content/uploads/2025/08/WFHResearch_updates_August2025.pdf
  - Sample size: July 2025 paid-days chart prints N = 248,859 (SWAA), which is far larger than one month and matches the deck's ">200,000 observations to date," so treat 248,859 as the sample behind that chart, not the July wave alone. Office-worker trend: N = 14,599. Facility comparison for January-July 2025: N = 26,933. The 13/60/27 slide did not have its own N in the extract. A separate February-July 2025 industry chart has N = 22,009 and is titled "Working from Home is Most Prevalent in Finance, Tech, and Professional and Business Services," but the sector percentages were not in the extracted text. Do not invent a tech percentage.
  - Caveat: days at home, not a private room, and not occupation = software developer. Employer plans on the same deck are "near 1.5 days per week"; that is plans for workers in general, not a developer share.
  - Quoted questions: "For each day last week, did you work a full day (6 or more hours), and if so where?" Office cut also uses "What type of facility best describes where you work?" and, for fully remote respondents, "If you had to come into the workplace, what type of facility would that be?"

- Knowledge workers, not developers. Leesman, home setting.
  - Claim: among employees Leesman surveyed, a majority of the 2023 home responses had a dedicated workroom or office at home, and most worked in more than one location.
  - Number: "In 2023, 58% of employees surveyed had a dedicated workroom or office at home, up 16 percentage points from 2020." "86% of the employees Leesman has surveyed over the past two years tell us they work in more than one location." A later Leesman article says the share with "a dedicated work room or office at home" was 38% of women and 44% of men in 2020, and 57% of women and 60% of men in 2024.
  - Year: 2023 figure in an article dated 30 May 2024; gender comparison stated for 2020 and 2024 in an article dated 10 November 2025.
  - Population: employees in Leesman client surveys. Not developers, unless a workplace happened to employ them. Not labeled as software engineers.
  - Geography: Leesman is multi-country. The 58% sentence does not give a country cut.
  - Source name: Leesman, "Making hybrid work"; Leesman, "Battle of the sexes: The gendered experience of hybrid working."
  - URLs: https://www.leesmanindex.com/articles/making-hybrid-work and https://www.leesmanindex.com/articles/battle-of-the-sexes-the-gendered-experience-of-hybrid-working
  - Sample size: not stated for the 58% or 86% sentences. The gender article describes a post-pandemic home dataset of n = 286,940 for Q1 2022 to Q2 2025. That window does not include 2020, so do not attach 286,940 to the 2020 percentages. It also does not explicitly say the 2024 gender split was computed on all 286,940.
  - Caveat: a dedicated room at home is the closest "private room" number found, and it is not a developer number. It is also not "I work in that room every day," and it is not "I can talk freely." The 2020-to-2023 change and the 2020-to-2024 gender series are both Leesman, but they are not the same table.
  - Quoted question: the articles use the response label "a dedicated work room or office at home" / "dedicated workroom or office at home." The full stem was not extracted.

### Inferences

- On Stack Overflow's own 2026 numbers, fully remote is a minority (34.8%) and fully in-person is also a minority (17.6%). A large hybrid middle exists. None of those categories tells you whether speech would disturb other people.
- The private-room evidence that does exist is about employees in workplace studies and about home offices in Leesman's client data. It cannot be quoted as "X% of developers have a private room."
- US office-facility workers in SWAA do a large share of full days at home (a 43% to 46% band in late 2024 through mid 2025). That is still not software engineers, and a day at home is not proof of a door that closes.

### Gaps

- No developer survey question of the form "do you work alone" or "do you work in a private room for part of the week" was found in Stack Overflow or JetBrains.
- JetBrains work-from-home or private-office share: not found. Do not use the 2023 "49% remote development" tool figure.
- BLS or American Time Use Survey table limited to software developers in a private room: not found. SWAA cites ATUS and the ACS for all-worker history; those occupation-filtered tables were not opened, and no software-developer percentage is reported here.
- Sector bars in the August 2025 SWAA deck name tech as a high work-from-home sector, but the percentages were not in the extract. Not found as a number in these notes.

## What share of software engineers sit in open offices versus private offices?

### Takeaway

Not found for software engineers. The workplace surveys that publish open-versus-private shares are of office workers. In Gensler's summer/fall 2020 US office-worker survey, 21% were in a mostly or totally private workplace and 69% were in some form of open office, while 47% said they would prefer a private environment. That sample is not software engineers, and it was taken during the pandemic.

### Cited Findings

- Gensler U.S. Workplace Survey, Summer/Fall 2020. Office workers, not developers.
  - Claim: most of these office workers were in an open setting; a minority were in a mostly or totally private setting; preference was more private than the then-current stock, and almost evenly split between preferring an open environment and preferring a private one.
  - Number: current or pre-COVID workplace, as labeled in the body of the PDF: totally open 11%, mostly open 16%, somewhat open 42%, mostly shared offices 10%, mostly private 19%, totally private 2%. Ideal: totally open 6%, mostly open 12%, somewhat open 28%, mostly shared 7%, mostly private 34%, totally private 13%. Prose: "46% of all U.S. workers would prefer an open work environment; currently 69% are in some form of open office." "47% of all U.S. workers would prefer a private work environment; currently only 21% are in a mostly or totally private workplace." Check: current open 11+16+42 = 69; current private 19+2 = 21; ideal open 6+12+28 = 46; ideal private 34+13 = 47. Shared offices are the remainder (10% current, 7% ideal) and are in neither the 69% nor the 21%.
  - Year: fielded 22 July to 24 August 2020.
  - Population: the methods box says 2,300+ US workers, full-time, at a company of 100 or more people, who had worked in an office before COVID-19. The slide headlines say "U.S. workers" and "U.S. workforce," which is broader than the methods. Not filtered to software engineers. The text says technology workers were less likely than sciences or consumer goods to prefer the office full time, but it does not give a technology-only open/private percentage.
  - Geography: United States.
  - Source name: Gensler Research Institute, "The Hybrid Future of Work," U.S. Workplace Survey Summer/Fall 2020.
  - URL: https://www.gensler.com/uploads/document/740/file/Gensler-US-Workplace-Survey-Summer-Fall-2020.pdf
  - Sample size: 2,300+ (methods). Not broken out by occupation in the extract.
  - Caveat: pandemic summer 2020. One summary graphic in the PDF extracted as the unordered-looking series 6%, 7%, 34%, 13%, 12%, 28%. That series does not match the prose totals. The labeled rows quoted above do match the prose, so those rows are the ones used. Do not call 21% or 69% a software-engineer share.
  - Quoted questions: "WHICH BEST DESCRIBES YOUR IDEAL WORKPLACE?" and "WHICH BEST DESCRIBES YOUR CURRENT OR PRE-COVID WORKPLACE?"

- Gensler, December 2023 blog on a 2023 US workplace survey. Still office workers. This is a preference for a mix of open and private space, not the share who currently have a private office.
  - Claim: 65% preferred "open areas for working with others with private spaces for individual work requiring deep focus."
  - Number: 65%. The blog says this preference was highest in Legal and lowest in Technology and Management Advisory, but it does not print the technology percentage.
  - Year: 2023 survey, blog dated 14 December 2023.
  - Population: "4,000+ workers across 10 industries in six U.S. cities." Office workers. Not a developer sample. Technology is one industry in the comparison, without a published share in the extract.
  - Geography: six US cities, unnamed in the extract.
  - Source name: Janet Pogue McLaurin, Gensler, "Open or Private? It's Time for a New Workplace Model."
  - URL: https://www.gensler.com/blog/open-or-private-time-for-a-new-workplace-model
  - Sample size: 4,000+.
  - Caveat: preference for a layout type, not current seating, and not software engineers. The technology ranking cannot be turned into "X% of developers sit in open offices."
  - Quoted question: not printed. The finding is the blog's summary of the ideal mix of open collaborative space and private focus space.

- Gensler 2013 U.S. Workplace Survey, rejected as an overall private-office rate.
  - The phrase "44% occupy private offices, 11% shared offices, 44% open plan desks" applies only to people in the 24% of respondents whose workplaces were classed as supporting both focus and collaboration ("balanced"). It is not the share of all workers, and it is not software engineers. Sample size was not in the extract. URL: https://www.gensler.com/uploads/document/337/file/2013_US_Workplace_Survey_07_15_2013.pdf
  - Do not cite 44% as the private-office share.

### Inferences

- Developer surveys and office-design surveys answer different questions. Stack Overflow can support a sentence about remote versus in-person developers. Gensler can support a sentence about US office workers at larger employers in 2020, many of whom sat in open settings and many of whom wanted more privacy. Joining those into "most software engineers sit in open offices" is not supported.
- Even the office-worker result cuts against a simple story that private offices are the norm. In that 2020 Gensler sample they were not.

### Gaps

- Open-office versus private-office share for software engineers: not found.
- A post-2020 Gensler or Leesman table that states the current percent of software engineers, or even of all office workers, in private offices versus open desks: not found in the pages opened. Leesman discusses "quiet rooms for working alone or in pairs" as a workplace feature people care about, without a share who have one, in the extracts reviewed.

## What is the measured cost of keyboard friction for short commands?

### Takeaway

The cost of a hotkey, of pressing Enter, or of moving back to the keyboard in order to confirm a short voice command while coding was probably never measured. What was measured is the general motor time to move a hand between mouse and keyboard, and the general time of a keystroke, in HCI models and in naturalistic logging of university computer users. Those times are fractions of a second. They are not a percent of a developer's day, and they are not a study of this product.

### Cited Findings

- Card, Moran, and Newell keystroke-level model, as quoted by a later measurement paper. The 1983 book was not re-opened.
  - Claim: the standard keystroke-level model uses 0.40 seconds for homing the hand between keyboard and mouse.
  - Number: 0.40 seconds. This is a model constant, not a 2026 field measurement of developers.
  - Year: the constant is from Card, Moran, and Newell 1983, as cited by Burns in 2016.
  - Population: the original constant comes from controlled HCI studies of computer users, not from a developer survey.
  - Geography: the original work was US academic HCI (Xerox PARC / Carnegie Mellon era). Not restated from the book here.
  - Source name: Stuart K. Card, Thomas P. Moran, and Allen Newell, The Psychology of Human-Computer Interaction (1983), via Marc Burns, "Using Naturalistic Typing to Update Architecture Typing Constants," ICCM 2016.
  - URL for the quotation of the 0.40 second constant: https://acs.ist.psu.edu/iccm2016/proceedings/burns2016iccm.pdf
  - Sample size: not re-established, because the 1983 source was not opened.
  - Caveat: a model operator for an expert doing a routine, error-free task. It is not the time to reorient from speaking a command to checking a paste in an IDE. Teaching notes repeat the same 0.40 second homing operator and about 0.20 seconds for an average keystroke; those notes were not used as the measurement source. See https://www.cs.umd.edu/~golbeck/INST631/KSM.pdf only as a secondary restatement, not as new data.
  - Quoted question: not a survey question. Burns: "The time to home took less time in either direction than the 0.40 s predicted by the KLM (Card et al., 1983)."

- Burns, ICCM 2016, naturalistic logs. Closest quantitative evidence on "switching back to the keyboard."
  - Claim: in ordinary computer use, the mean time from mouse to keyboard was 0.32 seconds, and from keyboard to mouse was 0.16 seconds, both under the 0.40 second model constant.
  - Number: mouse-to-keyboard mean 0.32 s (45,302 events); keyboard-to-mouse mean 0.16 s (51,989 events); 97,291 homing events in total.
  - Year: 2016 conference paper. Logs were of then-current daily use; the collection year inside the study is not separately stated in the extract.
  - Population: 45 people in one university environment: undergraduate students, office staff, professors, and graduate students. The paper says they are not a broad public sample. They are not identified as professional software engineers. A leading "170" in the text extract sits on its own line and is treated as a page number, not a headcount.
  - Geography: Pennsylvania State University context (College of IST).
  - Source name: Marc Burns, "Using Naturalistic Typing to Update Architecture Typing Constants."
  - URL: https://acs.ist.psu.edu/iccm2016/proceedings/burns2016iccm.pdf
  - Sample size: 45 participants. The introduction says 219 sessions and 1,865 hours. The results section says 1,816 hours and over 1.5 million keystrokes. Both hour figures are in the paper; they are not reconciled here.
  - Caveat: this is every kind of computer task, with an anonymizing logger, not coding and not a voice-command confirm. The author notes that very short times may be one hand still on the keyboard while the other uses the mouse, which pulls the mean down. Direction matters: coming back to the keyboard (0.32 s) was slower than leaving it (0.16 s). Do not turn 0.32 seconds into a percentage of the workday.
  - Quoted question: not a survey. Operational definition: homing is moving the hand from keyboard to mouse or mouse to keyboard.

- Burns master's thesis, 2023, catalog abstract only. The full thesis PDF was not opened. Later and larger than the 2016 paper, and the summary number does not match the 2016 directional means. Do not average them.
  - Claim: a later naturalistic sample produced a homing time around 360 milliseconds and a continuous-typing keystroke time around 245 milliseconds.
  - Number: the abstract prints "Homing to mouse and keyboard (n=59k, 358-362 ms)" and typing " (n=75K, 244-247 ms)". The abstract does not say whether 358-362 is a confidence interval or two directions. It is reported here only as printed.
  - Year: defense date 16 March 2023.
  - Population: participants recording normal computer use. The abstract does not say they are developers.
  - Geography: Pennsylvania State University thesis.
  - Source name: Marc Burns, "Updating and Extending the Keystroke Model with a Naturalistic Study," M.S. thesis.
  - URL: https://etda.libraries.psu.edu/catalog/26637mtb137
  - Sample size: participants n = 83 in the abstract. Homing events n = 59k. The full thesis was not opened, so the methods were not checked.
  - Caveat: "survey exists but I could not open it" applies to the full thesis. Only the catalog abstract was read. The 358-362 ms band is not the same statistic as the 2016 means of 0.16 s and 0.32 s.
  - Quoted question: none. It is a log study.

- Omanson, Miller, Young, and Schwantes, 2010. A lab comparison of menu-by-mouse, menu-by-keyboard-shortcut, and toolbar-by-mouse. No millisecond means were extracted, so none are stated.
  - Claim: across 90 trials, the toolbar-and-mouse method was fastest overall, and the menu-and-keyboard method improved the most with practice. The authors say their participants started with the hand already on the right device, and they cite the 0.40 second homing constant as something that could change a real-world comparison. They also cite Lane, Napier, Peres, and Sandor (2005) as finding simple key sequences faster than mouse menus.
  - Number: no trial time is reported here, because the extracted text of the paper did not include the means. Do not invent one. The only quantitative operator they restate is the 0.40 second homing estimate from Card, Moran, and Newell.
  - Year: 2010 (Human Factors and Ergonomics Society annual meeting paper; a PDF copy is dated in search metadata around that period).
  - Population: university students in a lab task, paid with a $5 gift card. Not developers at work.
  - Geography: not stated beyond the authors' affiliations in the extract.
  - Source name: Richard C. Omanson, Craig S. Miller, Elizabeth Young, and David Schwantes, "Comparison of Mouse and Keyboard Efficiency."
  - URLs: http://facweb.cs.depaul.edu/sjost/csc423/examples/anova/efficiency.pdf and https://journals.sagepub.com/doi/10.1177/154193121005400612
  - Sample size: not extracted. The design was 90 trials, scored in blocks of 30.
  - Caveat: hands were pre-positioned, so this study removes the homing cost it discusses. It does not measure Enter, a push-to-talk key, or returning from a voice preview to the keyboard.
  - Quoted question: not a survey.

- Lane, Napier, Peres, and Sandor, 2005, "Hidden costs of graphical user interfaces: Failure to make the transition from menus and icon toolbars to keyboard shortcuts," International Journal of Human-Computer Interaction. Named by Omanson. The paper was not opened, so no seconds and no percent are taken from it.

### Inferences

- If a report needs a number for "the tax of going back to the keyboard," the honest measured quantity is a few tenths of a second in general computer use, with the 1983 planning constant at 0.40 seconds and a 2016 naturalistic mouse-to-keyboard mean at 0.32 seconds. That does not show that hotkeys or Enter are a meaningful share of coding time.
- It also does not show the opposite, that the friction is zero in the saywhen flow. Nobody measured the flow: look at a preview, decide to cancel or send, press a key, and resume typing a short command.
- Task-interruption research about how long it takes to resume an interrupted project is a different construct. It was not used, and it must not be relabeled as the cost of Enter or a hotkey.

### Gaps

- Measured cost of pressing Enter to submit a short command: probably never measured as a developer-coding outcome. Not found.
- Measured cost of a global hotkey to start or stop dictation, or of context-switching back to the keyboard after a hands-free paste: not found.
- A percent of developer time lost to keyboard friction: not found. Do not compute one from 0.32 seconds.

## Does any survey say developers want voice input?

### Takeaway

Not found. No Stack Overflow, JetBrains, GitHub, or academic population survey was found that asks developers whether they want voice input and then reports a share. The only "would you use this" material is the five-person SPEED study in 2006, and those participants mostly said no to daily use of that system. That is an anecdote with a sample size, not a market.

### Cited Findings

- Not found, after the searches listed at the end of these notes.
- The nearest negative anecdote is Begel, 2006, n = 5, already cited above: the participants would not use SPEED for daily coding; one might use voice at home if working alone. URL: https://andrewbegel.com/papers/speed-short-paper.pdf
- Nowrin and Vertanen, 2023, did not ask a representative sample whether they want a voice-coding product. They asked 24 people to speak code. URL: https://www.keithv.com/pub/progvoice/nowrin_progvoice.pdf
- Creed and Sarcar studied people who already needed voice coding. Desire in that group does not estimate desire among developers. URL: https://www.open-access.bcu.ac.uk/15942/1/Creed_Voice_Coding_Experiences_Disabled_Developers%20%28Author%20Accepted%20Manuscript%29.pdf

### Inferences

- The supported sentence is: there is no survey evidence that developers want voice input, and the small lab record that exists is mixed to negative about daily use, with noise and being around other people named as reasons.
- Do not write "most developers want voice input," and do not write "most developers do not want it" as a survey result.

### Gaps

- A "want to use voice coding / dictation / speech-to-text" item on a developer survey: not found.
- Product pages and community posts for Talon, Serenade, Cursorless, and similar tools were not converted into user counts. No audited install or user total was opened.

### Searches that returned no usable number

- "Stack Overflow survey voice dictation"; "JetBrains survey speech recognition developers"; "academic survey voice programming developers"; "GitHub Octoverse voice coding survey." No adoption percentage.
- "site:survey.stackoverflow.co voice dictation speech"; "JetBrains developer ecosystem voice dictation"; "Stack Overflow survey disability options." No voice-while-working question with a share. The 2026 question preview did not show one in the text retrieved.
- "WebAIM screen reader survey speech recognition." Screen-reader surveys do not ask this. The 2013 motor survey has speech-recognition numbers and was rejected as the wrong population.
- "survey developers want voice coding." Not found.
- "JetBrains developer ecosystem remote work percent" and "site:jetbrains.com work from home share." No home-versus-office or private-room share. The 49% remote-development figure is a different question.
- "developers private office open plan survey" as a software-engineer statistic. Not found. Gensler and Leesman figures are office workers or client employees.
- "Gensler workplace survey private office percent" for a software-engineer cut, and for a clearly labeled post-2020 current seating share. The 2020 office-worker typology was found; a software-engineer cut was not.
- "WFH Research office workers days home" does have office-facility rates, but not a software-developer rate. Tech-sector bar values were not in the extract.
- "keyboard context switch keyboard interruption study" and "hotkey menu selection time study" as the cost of Enter, a dictation hotkey, or returning to the keyboard for a short coding command. Not found. General homing times were found instead.
- Lane, Napier, Peres, and Sandor 2005 was not opened, so it contributed no number.
- BLS / ATUS occupation filter for software developers in a private room: not found.
