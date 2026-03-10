package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/bwmarrin/discordgo"
)

// ChannelMessages holds all messages for one channel.
type ChannelMessages struct {
	ChannelID   string    `json:"channel_id"`
	ChannelName string    `json:"channel_name"`
	Messages    []Message `json:"messages"`
}

// GuildDump holds all channel logs for one guild.
type GuildDump struct {
	GuildID   string            `json:"guild_id"`
	GuildName string            `json:"guild_name"`
	Channels  []ChannelMessages `json:"channels"`
}

// Message is a single Discord message.
type Message struct {
	ID        string `json:"id"`
	Author    string `json:"author"`
	AuthorID  string `json:"author_id"`
	Content   string `json:"content"`
	Timestamp string `json:"timestamp"`
}

func main() {
	token := os.Getenv("DISCORD_TOKEN")
	if token == "" {
		token = os.Getenv("DISCORD_KEY")
	}
	if token == "" {
		log.Fatal("Set DISCORD_TOKEN or DISCORD_KEY environment variable")
	}

	dg, err := discordgo.New("Bot " + token)
	if err != nil {
		log.Fatalf("Failed to create session: %v", err)
	}

	dg.Identify.Intents = discordgo.IntentsGuilds |
		discordgo.IntentsGuildMessages |
		discordgo.IntentsMessageContent

	if err := dg.Open(); err != nil {
		log.Fatalf("Failed to connect: %v", err)
	}
	defer dg.Close()

	// Wait a moment for the ready event to populate state
	time.Sleep(2 * time.Second)

	guilds := dg.State.Guilds
	if len(guilds) == 0 {
		// Fallback: fetch via API
		guilds2, err := dg.UserGuilds(200, "", "", false)
		if err != nil {
			log.Fatalf("Failed to get guilds: %v", err)
		}
		log.Printf("Found %d guild(s) via API", len(guilds2))
		for _, g := range guilds2 {
			guild, err := dg.Guild(g.ID)
			if err != nil {
				log.Printf("Skipping guild %s: %v", g.ID, err)
				continue
			}
			dumpGuild(dg, guild)
		}
		return
	}

	log.Printf("Found %d guild(s) via state cache", len(guilds))
	for _, g := range guilds {
		dumpGuild(dg, g)
	}
}

func dumpGuild(dg *discordgo.Session, guild *discordgo.Guild) {
	log.Printf("Processing guild: %s (%s)", guild.Name, guild.ID)

	channels, err := dg.GuildChannels(guild.ID)
	if err != nil {
		log.Printf("  Failed to get channels: %v", err)
		return
	}

	dump := GuildDump{
		GuildID:   guild.ID,
		GuildName: guild.Name,
	}

	for _, ch := range channels {
		// Only text channels
		if ch.Type != discordgo.ChannelTypeGuildText {
			continue
		}

		log.Printf("  Channel: #%s (%s)", ch.Name, ch.ID)
		msgs := fetchAllMessages(dg, ch.ID)
		log.Printf("    %d messages", len(msgs))

		cm := ChannelMessages{
			ChannelID:   ch.ID,
			ChannelName: ch.Name,
		}
		for _, m := range msgs {
			cm.Messages = append(cm.Messages, Message{
				ID:        m.ID,
				Author:    m.Author.Username,
				AuthorID:  m.Author.ID,
				Content:   m.Content,
				Timestamp: m.Timestamp.Format(time.RFC3339),
			})
		}
		dump.Channels = append(dump.Channels, cm)
	}

	// Write JSON file
	filename := fmt.Sprintf("messages_%s.json", guild.ID)
	data, err := json.MarshalIndent(dump, "", "  ")
	if err != nil {
		log.Printf("  Failed to marshal JSON: %v", err)
		return
	}
	if err := os.WriteFile(filename, data, 0644); err != nil {
		log.Printf("  Failed to write %s: %v", filename, err)
		return
	}
	log.Printf("  Wrote %s", filename)
}

// fetchAllMessages pages through the channel history, oldest to newest.
func fetchAllMessages(dg *discordgo.Session, channelID string) []*discordgo.Message {
	var all []*discordgo.Message
	beforeID := ""

	for {
		msgs, err := dg.ChannelMessages(channelID, 100, beforeID, "", "")
		if err != nil {
			log.Printf("    Error fetching messages: %v", err)
			break
		}
		if len(msgs) == 0 {
			break
		}

		all = append(all, msgs...)
		beforeID = msgs[len(msgs)-1].ID

		// Rate limit courtesy
		time.Sleep(500 * time.Millisecond)
	}

	// Reverse so oldest is first
	for i, j := 0, len(all)-1; i < j; i, j = i+1, j-1 {
		all[i], all[j] = all[j], all[i]
	}

	return all
}
