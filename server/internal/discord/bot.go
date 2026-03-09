package discord

import (
	"context"
	"fmt"
	"log"
	"strings"
	"sync"
	"time"

	"github.com/bwmarrin/discordgo"
	"github.com/google/uuid"
	"github.com/secretarybird/server/internal/graph"
	"github.com/secretarybird/server/internal/models"
	"github.com/secretarybird/server/internal/openclaw"
	"github.com/secretarybird/server/internal/ws"
)

// Bot is the Discord bot for Secretarybird.
// It monitors channels, DMs follow-ups, runs daily check-ins,
// and collects messages for hourly batch processing.
type Bot struct {
	session       *discordgo.Session
	guildIDs      []string
	batchInterval time.Duration

	bridge *openclaw.Bridge
	fuseki *graph.FusekiClient
	hub    *ws.Hub

	// Message buffer for batch processing
	mu       sync.Mutex
	msgBuf   []bufferedMessage
	stopCh   chan struct{}
}

type bufferedMessage struct {
	guildID   string
	channelID string
	authorID  string
	author    string
	content   string
	timestamp time.Time
}

// NewBot creates a Discord bot. Does not connect until Start() is called.
func NewBot(token string, guildIDs []string, batchMinutes int, bridge *openclaw.Bridge, fuseki *graph.FusekiClient, hub *ws.Hub) (*Bot, error) {
	if token == "" {
		return nil, fmt.Errorf("DISCORD_TOKEN not set")
	}

	session, err := discordgo.New("Bot " + token)
	if err != nil {
		return nil, fmt.Errorf("failed to create Discord session: %w", err)
	}

	session.Identify.Intents = discordgo.IntentsGuildMessages |
		discordgo.IntentsDirectMessages |
		discordgo.IntentsMessageContent

	return &Bot{
		session:       session,
		guildIDs:      guildIDs,
		batchInterval: time.Duration(batchMinutes) * time.Minute,
		bridge:        bridge,
		fuseki:        fuseki,
		hub:           hub,
		stopCh:        make(chan struct{}),
	}, nil
}

// Start connects the bot to Discord and begins monitoring.
func (b *Bot) Start() error {
	b.session.AddHandler(b.onMessageCreate)
	b.session.AddHandler(b.onReady)

	if err := b.session.Open(); err != nil {
		return fmt.Errorf("failed to connect to Discord: %w", err)
	}

	// Start batch processing loop
	go b.batchLoop()

	log.Printf("[discord] bot connected")
	return nil
}

// Stop disconnects the bot.
func (b *Bot) Stop() {
	close(b.stopCh)
	if b.session != nil {
		b.session.Close()
	}
	log.Printf("[discord] bot disconnected")
}

// SendDM sends a direct message to a Discord user.
func (b *Bot) SendDM(userID, message string) error {
	channel, err := b.session.UserChannelCreate(userID)
	if err != nil {
		return fmt.Errorf("failed to create DM channel: %w", err)
	}

	_, err = b.session.ChannelMessageSend(channel.ID, message)
	if err != nil {
		return fmt.Errorf("failed to send DM: %w", err)
	}

	log.Printf("[discord] sent DM to %s", userID)
	return nil
}

// SendFollowUp delivers a follow-up question to a person via Discord DM.
func (b *Bot) SendFollowUp(discordUserID string, followUp *models.FollowUp) error {
	msg := followUp.Question
	if followUp.Context != "" {
		msg = followUp.Context + "\n\n" + followUp.Question
	}
	return b.SendDM(discordUserID, msg)
}

// SendDailyCheckin sends the daily check-in message to a person.
func (b *Bot) SendDailyCheckin(discordUserID, personName string, openQuestions []models.OpenQuestion) error {
	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("Hey %s! What are you working on today?\n", personName))

	if len(openQuestions) > 0 {
		sb.WriteString("\nI also have some questions for you:\n")
		for i, q := range openQuestions {
			sb.WriteString(fmt.Sprintf("%d. %s\n", i+1, q.Question))
		}
	}

	return b.SendDM(discordUserID, sb.String())
}

// onReady is called when the bot connects to Discord.
func (b *Bot) onReady(s *discordgo.Session, r *discordgo.Ready) {
	log.Printf("[discord] logged in as %s#%s", r.User.Username, r.User.Discriminator)
	s.UpdateGameStatus(0, "Watching for tasks")
}

// onMessageCreate handles every message the bot can see.
func (b *Bot) onMessageCreate(s *discordgo.Session, m *discordgo.MessageCreate) {
	// Ignore own messages
	if m.Author.ID == s.State.User.ID {
		return
	}

	// Check if this is a DM (direct reply to the bot)
	channel, err := s.Channel(m.ChannelID)
	if err != nil {
		return
	}

	if channel.Type == discordgo.ChannelTypeDM {
		// Direct messages get processed immediately
		go b.handleDirectMessage(m)
		return
	}

	// Guild messages get buffered for batch processing
	b.mu.Lock()
	b.msgBuf = append(b.msgBuf, bufferedMessage{
		guildID:   m.GuildID,
		channelID: m.ChannelID,
		authorID:  m.Author.ID,
		author:    m.Author.Username,
		content:   m.Content,
		timestamp: m.Timestamp,
	})
	b.mu.Unlock()
}

// handleDirectMessage processes a DM to the bot immediately.
// This could be a reply to a follow-up question or a direct command.
func (b *Bot) handleDirectMessage(m *discordgo.MessageCreate) {
	log.Printf("[discord] DM from %s: %s", m.Author.Username, m.Content)

	// Try to match this to an open question
	// For now, use OpenClaw to understand the message and route it
	ctx := context.Background()
	response, err := b.bridge.Chat(ctx, fmt.Sprintf(
		"A team member named %s sent this direct message in response to a follow-up question: \"%s\"\n\nInterpret this as a response and generate a brief acknowledgment.",
		m.Author.Username, m.Content), nil)
	if err != nil {
		log.Printf("[discord] OpenClaw error for DM: %v", err)
		b.SendDM(m.Author.ID, "Got it, thanks! I'll update the records.")
		return
	}

	b.SendDM(m.Author.ID, response)

	// Broadcast the interaction to connected clients
	b.hub.BroadcastMessage(models.WSMessage{
		Type:    "dm_response",
		Content: fmt.Sprintf("%s replied: %s", m.Author.Username, m.Content),
	})
}

// batchLoop runs the hourly batch processing cycle.
func (b *Bot) batchLoop() {
	ticker := time.NewTicker(b.batchInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			b.processBatch()
		case <-b.stopCh:
			return
		}
	}
}

// processBatch takes all buffered messages and sends them to OpenClaw for analysis.
func (b *Bot) processBatch() {
	b.mu.Lock()
	if len(b.msgBuf) == 0 {
		b.mu.Unlock()
		return
	}
	messages := b.msgBuf
	b.msgBuf = nil
	b.mu.Unlock()

	log.Printf("[discord] processing batch of %d messages", len(messages))

	// Build a conversation log for OpenClaw
	var sb strings.Builder
	sb.WriteString("Discord messages from the last batch period:\n\n")
	for _, msg := range messages {
		sb.WriteString(fmt.Sprintf("[%s] %s: %s\n",
			msg.timestamp.Format("15:04"),
			msg.author,
			msg.content))
	}

	// Send to OpenClaw for analysis
	ctx := context.Background()
	result, err := b.bridge.Analyze(ctx, sb.String(), "Discord channel monitoring batch")
	if err != nil {
		log.Printf("[discord] batch analysis failed: %v", err)
		return
	}

	log.Printf("[discord] batch analysis complete: %s", truncate(result, 200))

	// Store the messages in the graph
	for _, msg := range messages {
		m := &models.Message{
			ID:          uuid.New(),
			SourceIngest: "discord-batch",
			Author:      fmt.Sprintf("discord:%s", msg.authorID),
			Content:     msg.content,
			Timestamp:   &msg.timestamp,
			Confidence:  1.0, // Direct from Discord, high confidence
		}
		if err := b.fuseki.StoreMessage(ctx, m); err != nil {
			log.Printf("[discord] failed to store message: %v", err)
		}
	}

	// Broadcast batch results to connected clients
	b.hub.BroadcastMessage(models.WSMessage{
		Type:    "batch_complete",
		Content: fmt.Sprintf("Processed %d Discord messages", len(messages)),
		Data:    result,
	})
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}
